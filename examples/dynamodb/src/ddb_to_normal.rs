use embedded_io::{Read as IoRead, Write as IoWrite};
use rjiter::jiter::Peek;
use rjiter::RJiter;
use scan_json::{scan, Action, EndAction, Options, StreamOp};
use scan_json::matcher::StructuralPseudoname;
use scan_json::stack::ContextIter;
use core::cell::RefCell;
use u8pool::U8Pool;
use crate::ConversionError;

/// What phase of parsing we're in
#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    ExpectingField,        // Expecting a field key
    ExpectingTypeDesc,     // Expecting type descriptor object (after field key)
    ExpectingTypeKey,      // Inside type descriptor, expecting type key
    ExpectingValue,        // After type key, expecting the value
}

/// Type descriptor being processed
#[derive(Debug, Clone, Copy, PartialEq)]
enum TypeDesc {
    S, N, Bool, Null,
    SS, NS,  // Sets
    L, M,    // Nested containers
}

/// Check if we're DIRECTLY inside an L (list) container (not nested in M inside L)
/// This is used to determine the correct phase after consuming a value
fn in_list_from_context(context: ContextIter) -> bool {
    let mut ctx = context.clone();
    // Check if the FIRST item in context is #array
    if let Some(first) = ctx.next() {
        if first == b"#array" {
            // We're directly in an array, check if it's an L array
            if let Some(array_key) = ctx.next() {
                return array_key == b"L";
            }
        }
    }
    false
}

/// Check if we're ending an L array (context is at the array level, not inside it)
fn ending_l_array_from_context(context: ContextIter) -> bool {
    let mut ctx = context.clone();
    // First item in context should be the key of the array
    if let Some(key) = ctx.next() {
        return key == b"L";
    }
    false
}

/// Check if we're ending an M object (context is at the object level, not inside it)
/// Need to distinguish from a field named "M" whose type descriptor is ending
fn ending_m_object_from_context(context: ContextIter) -> bool {
    let mut ctx = context.clone();
    // First item in context should be the key of the object
    if let Some(key) = ctx.next() {
        if key == b"M" {
            // Check parent - if parent is #array (L array), "M" is a type key
            // If parent is also a type key, then "M" is a field name, not ending M object
            if let Some(parent) = ctx.next() {
                match parent {
                    b"#array" => {
                        // In an array, check if it's an L array
                        if let Some(array_parent) = ctx.next() {
                            return array_parent == b"L";
                        }
                        return false;
                    }
                    // If parent is any type key, this "M" is a field name inside that type's value
                    b"S" | b"N" | b"BOOL" | b"NULL" | b"SS" | b"NS" | b"BS" | b"L" => {
                        // Parent is a type key, so this "M" is a field name
                        return false;
                    }
                    b"M" => {
                        // Parent is "M" - could be a type key OR a field name
                        // Need to check grandparent and possibly great-grandparent
                        if let Some(grandparent) = ctx.next() {
                            match grandparent {
                                b"M" => {
                                    // Grandparent is also "M" - deeply nested M case
                                    // Check great-grandparent to determine the pattern
                                    if let Some(great_gp) = ctx.next() {
                                        match great_gp {
                                            // If great-grandparent IS a type key or marker, then:
                                            // parent "M" is a field name inside another M's type descriptor
                                            b"S" | b"N" | b"BOOL" | b"NULL" | b"SS" | b"NS" | b"BS" | b"L" | b"M" | b"#array" | b"#top" => {
                                                return false;
                                            }
                                            // great-grandparent is a field name, so:
                                            // grandparent="M" is type key, parent="M" is field, current="M" is type
                                            _ => {
                                                return true;
                                            }
                                        }
                                    }
                                    // No great-grandparent, conservatively assume we're closing a type descriptor
                                    return false;
                                }
                                // If grandparent is any other type key or special marker
                                b"S" | b"N" | b"BOOL" | b"NULL" | b"SS" | b"NS" | b"BS" | b"L" | b"#array" | b"#top" => {
                                    // Parent "M" is a field name, current "M" is a type key
                                    return true;
                                }
                                // Otherwise grandparent is a regular field name
                                _ => {
                                    // Parent "M" is the type key for grandparent field
                                    // Current "M" is a field name inside that M value
                                    return false;
                                }
                            }
                        }
                        // No grandparent, assume parent "M" is a type key
                        return false;
                    }
                    // For any other parent (regular field name), "M" is a type key
                    _ => {
                        return true;
                    }
                }
            } else {
                // No parent, so "M" is a type key
                return true;
            }
        }
    }
    false
}

pub struct DdbConverter<'a, 'workbuf, W: IoWrite> {
    writer: &'a mut W,
    pending_comma: bool,
    pretty: bool,
    depth: usize,
    current_field: Option<&'workbuf [u8]>,
    has_item_wrapper: Option<bool>, // None = unknown, Some(true) = has Item, Some(false) = no Item
    last_error: Option<ConversionError>, // Stores detailed error information

    phase: Phase,
    current_type: Option<TypeDesc>,
    skip_next_object: bool, // Skip treating next object as type descriptor (for Item value)
    type_descriptor_depth: usize, // Nesting depth of type descriptors (0 = not in type descriptor)
    m_depth: usize, // Nesting depth of M objects (for distinguishing M from field named "M")

    // Cached context information (set in find_action/find_end_action)
    current_in_list: bool, // True if currently in an L container
}

impl<'a, 'workbuf, W: IoWrite> DdbConverter<'a, 'workbuf, W> {
    fn new(writer: &'a mut W, pretty: bool) -> Self {
        Self {
            writer,
            pending_comma: false,
            pretty,
            depth: 0,
            current_field: None,
            has_item_wrapper: None,
            last_error: None,
            phase: Phase::ExpectingField,
            current_type: None,
            skip_next_object: false,
            type_descriptor_depth: 0,
            m_depth: 0,
            current_in_list: false,
        }
    }

    fn store_rjiter_error(&mut self, error: rjiter::Error, position: usize, context: &'static str) {
        self.last_error = Some(ConversionError::RJiterError {
            kind: error.error_type,
            position,
            context,
        });
    }

    #[allow(dead_code)]
    fn store_io_error(&mut self, kind: embedded_io::ErrorKind, position: usize, context: &'static str) {
        self.last_error = Some(ConversionError::IOError {
            kind,
            position,
            context,
        });
    }

    fn store_parse_error(&mut self, position: usize, context: &'static str, unknown_type_bytes: Option<&[u8]>) {
        let unknown_type = if let Some(bytes) = unknown_type_bytes {
            let len = bytes.len().min(32);
            let mut buffer = [0u8; 32];
            buffer[..len].copy_from_slice(&bytes[..len]);
            Some((buffer, len))
        } else {
            None
        };

        self.last_error = Some(ConversionError::ParseError {
            position,
            context,
            unknown_type,
        });
    }

    fn write(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
    }

    fn write_comma(&mut self) {
        if self.pending_comma {
            self.write(b",");
            self.newline();
            self.pending_comma = false;
        }
    }

    fn newline(&mut self) {
        if self.pretty {
            self.write(b"\n");
        }
    }

    fn indent(&mut self) {
        if self.pretty {
            for _ in 0..self.depth {
                self.write(b"  ");
            }
        }
    }
}

type DdbBaton<'a, 'workbuf, W> = &'a RefCell<DdbConverter<'a, 'workbuf, W>>;

/// Handle the root object - just enter it, don't write anything yet
fn on_root_object_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Don't write anything - we'll write when we know if there's an Item wrapper
    let _conv = baton.borrow();
    StreamOp::None
}

/// Handle the "Item" key at root - prepare for Item value object
fn on_item_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.has_item_wrapper = Some(true);
    conv.skip_next_object = true; // Next object is the Item value, not a type descriptor
    StreamOp::None
}

/// Handle the start of Item value object - write opening brace and enter field-expecting phase
fn on_item_value_object_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.write(b"{");
    conv.newline();
    conv.depth = 1;
    conv.pending_comma = false;
    conv.phase = Phase::ExpectingField;
    StreamOp::None
}

/// Handle the end of Item object - write closing brace and newline for JSONL
fn on_item_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.write(b"}");
    conv.write(b"\n");
    Ok(())
}

/// Handle a field key - write the field name and prepare for type descriptor
fn on_field_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let field_name = {
        let conv = baton.borrow();
        conv.current_field.expect("current_field should be set").to_vec()
    };

    let mut conv = baton.borrow_mut();

    // If we haven't seen Item wrapper yet, this is the first field (no Item wrapper)
    if conv.has_item_wrapper.is_none() {
        conv.write(b"{");
        conv.newline();
        conv.depth = 1;
        conv.has_item_wrapper = Some(false);
    }

    conv.write_comma();
    conv.indent();
    conv.write(b"\"");
    conv.write(&field_name);
    conv.write(b"\":");
    conv.pending_comma = false;
    conv.phase = Phase::ExpectingTypeDesc;

    StreamOp::None
}

/// Handle the start of a type descriptor object
fn on_type_descriptor_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();

    // In L arrays, write comma before element
    if conv.current_in_list {
        conv.write_comma();
    }

    conv.type_descriptor_depth += 1;
    conv.phase = Phase::ExpectingTypeKey;
    StreamOp::None
}

/// Handle a type key and transition to expecting value
fn on_type_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let type_key = {
        let conv = baton.borrow();
        conv.current_field.expect("current_field should be set for type key").to_vec()
    };

    let mut conv = baton.borrow_mut();

    let type_desc = match type_key.as_slice() {
        b"S" | b"B" => TypeDesc::S,
        b"N" => TypeDesc::N,
        b"BOOL" => TypeDesc::Bool,
        b"NULL" => TypeDesc::Null,
        b"SS" | b"BS" => TypeDesc::SS,
        b"NS" => TypeDesc::NS,
        b"L" => TypeDesc::L,
        b"M" => TypeDesc::M,
        _ => {
            conv.store_parse_error(
                0,
                "Invalid DynamoDB JSON format: unknown type descriptor",
                Some(&type_key),
            );
            return StreamOp::Error("Unknown type descriptor");
        }
    };

    conv.current_type = Some(type_desc);
    conv.phase = Phase::ExpectingValue;
    StreamOp::None
}

// Type descriptor value handlers

fn on_string_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking S (string) type value");
            return StreamOp::Error("Failed to peek string value");
        }
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string value for S type");
    }

    let mut conv = baton.borrow_mut();
    conv.write(b"\"");
    if let Err(e) = rjiter.write_long_bytes(conv.writer) {
        conv.store_rjiter_error(e, position, "writing S (string) type value");
        return StreamOp::Error("Failed to write string value");
    }
    conv.write(b"\"");
    conv.pending_comma = true;

    // After value, return to field or type descriptor level
    conv.current_type = None;
    conv.phase = if conv.current_in_list { Phase::ExpectingTypeDesc } else { Phase::ExpectingField };
    StreamOp::ValueIsConsumed
}

fn on_number_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking N (number) type value");
            return StreamOp::Error("Failed to peek number value");
        }
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string value for N (number) type");
    }

    let mut conv = baton.borrow_mut();
    if let Err(e) = rjiter.write_long_bytes(conv.writer) {
        conv.store_rjiter_error(e, position, "writing N (number) type value");
        return StreamOp::Error("Failed to write number value");
    }
    conv.pending_comma = true;

    // After value, return to field or type descriptor level
    conv.current_type = None;
    conv.phase = if conv.current_in_list { Phase::ExpectingTypeDesc } else { Phase::ExpectingField };
    StreamOp::ValueIsConsumed
}

fn on_bool_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking BOOL type value");
            return StreamOp::Error("Failed to peek boolean value");
        }
    };

    let mut conv = baton.borrow_mut();
    match peek {
        Peek::True => {
            if let Err(e) = rjiter.known_bool(peek) {
                conv.store_rjiter_error(e, position, "consuming BOOL type value (true)");
                return StreamOp::Error("Failed to consume true value");
            }
            conv.write(b"true");
        }
        Peek::False => {
            if let Err(e) = rjiter.known_bool(peek) {
                conv.store_rjiter_error(e, position, "consuming BOOL type value (false)");
                return StreamOp::Error("Failed to consume false value");
            }
            conv.write(b"false");
        }
        _ => return StreamOp::Error("Expected boolean value for BOOL type"),
    }
    conv.pending_comma = true;

    // After value, return to field or type descriptor level
    conv.current_type = None;
    conv.phase = if conv.current_in_list { Phase::ExpectingTypeDesc } else { Phase::ExpectingField };
    StreamOp::ValueIsConsumed
}

fn on_null_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking NULL type value");
            return StreamOp::Error("Failed to peek NULL value");
        }
    };
    if peek != Peek::True {
        return StreamOp::Error("Expected true for NULL type");
    }
    if let Err(e) = rjiter.known_bool(peek) {
        baton.borrow_mut().store_rjiter_error(e, position, "consuming NULL type value");
        return StreamOp::Error("Failed to consume NULL value");
    }

    let mut conv = baton.borrow_mut();
    conv.write(b"null");
    conv.pending_comma = true;

    // After value, return to field or type descriptor level
    conv.current_type = None;
    conv.phase = if conv.current_in_list { Phase::ExpectingTypeDesc } else { Phase::ExpectingField };
    StreamOp::ValueIsConsumed
}

fn on_string_set_begin<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking SS/BS (string set) type value");
            return StreamOp::Error("Failed to peek string set value");
        }
    };
    if peek != Peek::Array {
        let mut conv = baton.borrow_mut();
        conv.store_parse_error(
            position,
            "Invalid DynamoDB JSON format: SS/BS type expects an array value",
            None,
        );
        return StreamOp::Error("Expected array value for SS/BS type");
    }

    let mut conv = baton.borrow_mut();
    conv.write(b"[");
    conv.pending_comma = false;
    // Stay in ExpectingValue, SS elements are atoms
    StreamOp::None
}

fn on_number_set_begin<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking NS (number set) type value");
            return StreamOp::Error("Failed to peek number set value");
        }
    };
    if peek != Peek::Array {
        let mut conv = baton.borrow_mut();
        conv.store_parse_error(
            position,
            "Invalid DynamoDB JSON format: NS type expects an array value",
            None,
        );
        return StreamOp::Error("Expected array value for NS type");
    }

    let mut conv = baton.borrow_mut();
    conv.write(b"[");
    conv.pending_comma = false;
    // Stay in ExpectingValue, NS elements are atoms
    StreamOp::None
}

fn on_list_begin<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking L (list) type value");
            return StreamOp::Error("Failed to peek list value");
        }
    };
    if peek != Peek::Array {
        let mut conv = baton.borrow_mut();
        conv.store_parse_error(
            position,
            "Invalid DynamoDB JSON format: L type expects an array value",
            None,
        );
        return StreamOp::Error("Expected array value for L type");
    }

    let mut conv = baton.borrow_mut();
    conv.write(b"[");
    conv.pending_comma = false;
    conv.phase = Phase::ExpectingTypeDesc;  // In L, we expect type descriptors
    StreamOp::None
}

fn on_map_begin<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking M (map) type value");
            return StreamOp::Error("Failed to peek map value");
        }
    };
    if peek != Peek::Object {
        let mut conv = baton.borrow_mut();
        conv.store_parse_error(
            position,
            "Invalid DynamoDB JSON format: M type expects an object value",
            None,
        );
        return StreamOp::Error("Expected object value for M type");
    }

    let mut conv = baton.borrow_mut();
    conv.write(b"{");
    conv.newline();
    conv.depth += 1;
    conv.pending_comma = false;
    conv.m_depth += 1;  // Track M nesting
    conv.phase = Phase::ExpectingField;  // In M, we expect field keys
    StreamOp::None
}

fn on_set_string_element<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking SS/BS (string set) element");
            return StreamOp::Error("Failed to peek string set element");
        }
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string in SS/BS set");
    }

    let mut conv = baton.borrow_mut();
    conv.write_comma();
    conv.write(b"\"");
    if let Err(e) = rjiter.write_long_bytes(conv.writer) {
        conv.store_rjiter_error(e, position, "writing SS/BS (string set) element");
        return StreamOp::Error("Failed to write string set element");
    }
    conv.write(b"\"");
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

fn on_set_number_element<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            baton.borrow_mut().store_rjiter_error(e, position, "peeking NS (number set) element");
            return StreamOp::Error("Failed to peek number set element");
        }
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string (number) in NS set");
    }

    let mut conv = baton.borrow_mut();
    conv.write_comma();
    if let Err(e) = rjiter.write_long_bytes(conv.writer) {
        conv.store_rjiter_error(e, position, "writing NS (number set) element");
        return StreamOp::Error("Failed to write number set element");
    }
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

/// Handle Object structural pseudoname
fn find_action_object<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    mut context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
    current_type: Option<TypeDesc>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Check for root object at #top
    if let Some(first) = context.next() {
        if first == b"#top" {
            return Some(on_root_object_begin);
        }
    }

    // Check if this is the Item value object
    {
        let mut conv = baton.borrow_mut();
        if conv.skip_next_object {
            conv.skip_next_object = false;
            drop(conv);
            return Some(on_item_value_object_begin);
        }
    }

    match phase {
        Phase::ExpectingTypeDesc => {
            return Some(on_type_descriptor_begin);
        }
        Phase::ExpectingValue => {
            // Check type - should be M
            if current_type == Some(TypeDesc::M) {
                return Some(on_map_begin);
            } else if current_type == Some(TypeDesc::L) {
                // L expects array, not object
                return Some(on_invalid_type_value_not_array);
            } else {
                // Other types (S, N, BOOL, NULL, SS, NS) expect primitives/arrays, not objects
                return Some(on_invalid_type_value_not_atom);
            }
        }
        _ => {}
    }

    None
}

fn find_action_key<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    mut context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    let key = context.next()?;

    // Check if this is "Item" key at root level (parent is #top)
    if key == b"Item" {
        if let Some(b"#top") = context.next() {
            let mut conv = baton.borrow_mut();
            #[allow(unsafe_code)]
            let key_slice: &'workbuf [u8] =
                unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(key) };
            conv.current_field = Some(key_slice);
            return Some(on_item_key);
        }
    }

    // Store the key
    let mut conv = baton.borrow_mut();
    #[allow(unsafe_code)]
    let key_slice: &'workbuf [u8] =
        unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(key) };
    conv.current_field = Some(key_slice);
    drop(conv);

    match phase {
        Phase::ExpectingField => Some(on_field_key),
        Phase::ExpectingTypeKey => Some(on_type_key),
        _ => None,
    }
}

/// Handle Array structural pseudoname
fn find_action_array<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    _context: ContextIter,
    _baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
    current_type: Option<TypeDesc>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    if phase == Phase::ExpectingValue {
        match current_type {
            Some(TypeDesc::SS) => return Some(on_string_set_begin),
            Some(TypeDesc::NS) => return Some(on_number_set_begin),
            Some(TypeDesc::L) => return Some(on_list_begin),
            Some(TypeDesc::M) => return Some(on_invalid_type_value_not_object),
            Some(TypeDesc::S) | Some(TypeDesc::N) | Some(TypeDesc::Bool) | Some(TypeDesc::Null) => {
                return Some(on_invalid_type_value_not_atom);
            }
            _ => {}
        }
    }

    None
}

/// Handle Atom structural pseudoname
fn find_action_atom<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    mut context: ContextIter,
    _baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
    current_type: Option<TypeDesc>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    if phase == Phase::ExpectingValue {
        match current_type {
            Some(TypeDesc::S) => return Some(on_string_value),
            Some(TypeDesc::N) => return Some(on_number_value),
            Some(TypeDesc::Bool) => return Some(on_bool_value),
            Some(TypeDesc::Null) => return Some(on_null_value),
            Some(TypeDesc::SS) | Some(TypeDesc::NS) => {
                // Check if we're inside an array (element) or at object level (invalid)
                if let Some(first) = context.next() {
                    if first == b"#array" {
                        // We're inside the array, this is a valid element
                        if current_type == Some(TypeDesc::SS) {
                            return Some(on_set_string_element);
                        } else {
                            return Some(on_set_number_element);
                        }
                    }
                }
                // Not inside an array, this is an error
                return Some(on_invalid_type_value_not_array);
            }
            Some(TypeDesc::L) => return Some(on_invalid_type_value_not_array),
            Some(TypeDesc::M) => return Some(on_invalid_type_value_not_object),
            _ => {}
        }
    }

    None
}

fn find_action<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Update cached context information
    {
        let mut conv = baton.borrow_mut();
        conv.current_in_list = in_list_from_context(context.clone());
    }

    let (phase, current_type) = {
        let conv = baton.borrow();
        (conv.phase, conv.current_type)
    };

    // Match on structural type and delegate to appropriate handler
    match structural {
        StructuralPseudoname::Object => find_action_object(context, baton, phase, current_type),
        StructuralPseudoname::None => find_action_key(context, baton, phase),
        StructuralPseudoname::Array => find_action_array(context, baton, phase, current_type),
        StructuralPseudoname::Atom => find_action_atom(context, baton, phase, current_type),
    }
}

fn on_invalid_type_value_not_array<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let type_name = match conv.current_type {
        Some(TypeDesc::SS) => "SS/BS",
        Some(TypeDesc::NS) => "NS",
        Some(TypeDesc::L) => "L",
        _ => "unknown",
    };
    conv.store_parse_error(
        0,
        "Invalid DynamoDB JSON format: type expects an array value",
        Some(type_name.as_bytes()),
    );
    StreamOp::Error("Expected array value for type")
}

fn on_invalid_type_value_not_object<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.store_parse_error(
        0,
        "Invalid DynamoDB JSON format: M type expects an object value",
        Some(b"M"),
    );
    StreamOp::Error("Expected object value for M type")
}

fn on_invalid_type_value_not_atom<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let type_name = match conv.current_type {
        Some(TypeDesc::S) => "S",
        Some(TypeDesc::N) => "N",
        Some(TypeDesc::Bool) => "BOOL",
        Some(TypeDesc::Null) => "NULL",
        _ => "unknown",
    };
    conv.store_parse_error(
        0,
        "Invalid DynamoDB JSON format: type expects a primitive value",
        Some(type_name.as_bytes()),
    );
    StreamOp::Error("Expected primitive value for type")
}

fn on_list_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.write(b"]");
    conv.pending_comma = true;

    // Ending L array - restore phase based on whether we're still in another L
    conv.phase = if conv.current_in_list { Phase::ExpectingTypeDesc } else { Phase::ExpectingField };
    conv.current_type = None;
    Ok(())
}

fn on_set_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.write(b"]");
    conv.pending_comma = true;

    // Ending SS/NS set - just clear type and return to field/typedesc level (phase already set by last element)
    conv.current_type = None;
    Ok(())
}

fn on_map_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;

    // Decrement M depth
    conv.m_depth = conv.m_depth.saturating_sub(1);

    // Restore phase based on whether we're in an L array
    conv.current_type = None;
    conv.phase = if conv.current_in_list { Phase::ExpectingTypeDesc } else { Phase::ExpectingField };
    Ok(())
}

fn on_type_descriptor_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    // Type descriptor ended, return to field or typedesc level
    conv.type_descriptor_depth = conv.type_descriptor_depth.saturating_sub(1);
    conv.phase = if conv.current_in_list { Phase::ExpectingTypeDesc } else { Phase::ExpectingField };
    Ok(())
}

fn on_list_element_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.pending_comma = true;
    Ok(())
}

fn on_root_object_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let conv = baton.borrow();
    // If there was no Item wrapper, the root object IS the record, so close it
    if conv.has_item_wrapper == Some(false) {
        drop(conv);
        on_item_end(baton)?;
    }
    // If there was an Item wrapper, the root object just wraps "Item", nothing to write
    Ok(())
}

fn find_end_action<'a, 'workbuf, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<EndAction<DdbBaton<'a, 'workbuf, W>>> {
    // Update cached context information
    {
        let mut conv = baton.borrow_mut();
        conv.current_in_list = in_list_from_context(context.clone());
    }

    let (phase, current_type, in_list, type_descriptor_depth, m_depth) = {
        let conv = baton.borrow();
        (conv.phase, conv.current_type, conv.current_in_list, conv.type_descriptor_depth, conv.m_depth)
    };

    // Match end of objects
    if structural == StructuralPseudoname::Object {
        match phase {
            Phase::ExpectingField => {
                // This could be the Item/root object or an M object
                // Check for M value ending FIRST, then type descriptors
                let ending_m = m_depth > 0 && ending_m_object_from_context(context.clone());

                if ending_m {
                    return Some(on_map_end);
                } else if type_descriptor_depth >= 1 {
                    return Some(on_type_descriptor_end);
                } else {
                    // depth == 0, not in type descriptor or M container
                    // Check if this is the Item value object or root object
                    let mut ctx = context.clone();
                    if let Some(first) = ctx.next() {
                        if first != b"#top" {
                            // Not the root object, this is the Item value object
                            return Some(on_item_end);
                        } else {
                            // Root object ending - check if no Item wrapper
                            let has_wrapper = baton.borrow().has_item_wrapper;
                            if has_wrapper == Some(false) {
                                // No Item wrapper, root object IS the record
                                return Some(on_item_end);
                            }
                            // has_wrapper == Some(true) or None - already handled elsewhere
                        }
                    }
                }
            }
            _ => {
                // For other phases, check if we're ending a type descriptor
                if type_descriptor_depth > 0 {
                    return Some(on_type_descriptor_end);
                }
            }
        }
    }

    // Match end of arrays
    if structural == StructuralPseudoname::Array {
        // Check if we're ending an L array (context key is "L")
        if ending_l_array_from_context(context.clone()) {
            return Some(on_list_end);
        }
        // Otherwise, check for SS/NS arrays by current_type
        match current_type {
            Some(TypeDesc::SS) | Some(TypeDesc::NS) => {
                // SS/NS arrays: phase is ExpectingValue while processing elements
                if phase == Phase::ExpectingValue {
                    return Some(on_set_end);
                }
            }
            _ => {}
        }
    }

    // Match end of type descriptor objects in L arrays
    if structural == StructuralPseudoname::Object {
        if phase == Phase::ExpectingTypeKey && in_list {
            // Type descriptor in L array ending
            return Some(on_list_element_end);
        }
    }

    None
}

/// Convert DynamoDB JSON to normal JSON in a streaming, allocation-free manner.
/// Supports JSONL format (newline-delimited JSON) - processes multiple JSON objects.
///
/// # Arguments
/// * `reader` - Input stream implementing `embedded_io::Read`
/// * `writer` - Output stream implementing `embedded_io::Write`
/// * `rjiter_buffer` - Buffer for rjiter to use (recommended: 4096 bytes)
/// * `context_buffer` - Buffer for scan_json context tracking (recommended: 2048 bytes)
/// * `pretty` - Whether to pretty-print the output
///
/// # Returns
/// `Ok(())` on success, or `Err(ConversionError)` with detailed error information on failure
pub fn convert_ddb_to_normal<R: IoRead, W: IoWrite>(
    reader: &mut R,
    writer: &mut W,
    rjiter_buffer: &mut [u8],
    context_buffer: &mut [u8],
    pretty: bool,
) -> Result<(), ConversionError> {
    let mut rjiter = RJiter::new(reader, rjiter_buffer);

    let converter = DdbConverter::new(writer, pretty);
    let baton = RefCell::new(converter);

    let mut context = U8Pool::new(context_buffer, 32)
        .map_err(|_| ConversionError::ScanError(
            scan_json::Error::InternalError {
                position: 0,
                message: "Failed to create context pool"
            }
        ))?;

    if let Err(e) = scan(
        find_action,
        find_end_action,
        &mut rjiter,
        &baton,
        &mut context,
        &Options::new(),
    ) {
        // Check if there's a stored detailed error in the baton
        let stored_error = baton.borrow().last_error.clone();
        if let Some(err) = stored_error {
            return Err(err);
        }
        // Otherwise return the scan error
        return Err(ConversionError::ScanError(e));
    }

    Ok(())
}
