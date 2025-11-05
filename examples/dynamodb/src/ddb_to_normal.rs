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
    ExpectingTypeKey,      // Expecting type key (after field key, or in L array)
    ExpectingValue,        // After type key, expecting the value (only for sets: SS, NS, BS)
}

/// How to handle "Item" key at top level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemWrapperMode {
    AsWrapper,   // Interpret "Item" at top level as a special wrapper
    AsField,     // Interpret "Item" at top level as a normal field
}

/// Type descriptor being processed (only for container types)
#[derive(Debug, Clone, Copy, PartialEq)]
enum TypeDesc {
    SS, NS,  // Sets
    L, M,    // Nested containers
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
    depth: usize, // JSON output nesting depth (for pretty-printing indentation and root level detection)
    current_field: Option<&'workbuf [u8]>,
    item_wrapper_mode: ItemWrapperMode, // How to handle "Item" key at top level
    last_error: Option<ConversionError>, // Stores detailed error information
    item_end_called: bool, // Track if on_item_end has been called to prevent duplicate calls

    phase: Phase,
    current_type: Option<TypeDesc>,
    m_depth: usize, // Nesting depth of M objects (for distinguishing M from field named "M")
    l_depth: usize, // Nesting depth of L arrays (for determining phase after literals)
}

impl<'a, 'workbuf, W: IoWrite> DdbConverter<'a, 'workbuf, W> {
    fn new(writer: &'a mut W, pretty: bool, item_wrapper_mode: ItemWrapperMode) -> Self {
        Self {
            writer,
            pending_comma: false,
            pretty,
            depth: 0,
            current_field: None,
            item_wrapper_mode,
            last_error: None,
            item_end_called: false,
            phase: Phase::ExpectingField,
            current_type: None,
            m_depth: 0,
            l_depth: 0,
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

    fn write_comma_if_pending(&mut self) {
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

/// Handle the end of Item object - write closing brace and newline for JSONL
fn on_item_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    // Prevent duplicate calls
    if conv.item_end_called {
        return Ok(());
    }
    conv.item_end_called = true;
    conv.newline();
    conv.write(b"}");
    conv.write(b"\n");
    Ok(())
}

/// Handle root object beginning - write opening brace
fn on_root_object_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.write(b"{");
    conv.newline();
    conv.depth = 1;
    StreamOp::None
}

/// Handle a field key - write the field name and prepare for type descriptor
fn on_field_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let field_name = {
        let conv = baton.borrow();
        conv.current_field.expect("current_field should be set").to_vec()
    };

    let mut conv = baton.borrow_mut();

    conv.write_comma_if_pending();
    conv.indent();
    conv.write(b"\"");
    conv.write(&field_name);
    conv.write(b"\":");
    conv.pending_comma = false;
    conv.phase = Phase::ExpectingTypeKey;

    StreamOp::None
}

/// Generic helper for writing string-based values (S/B/N types and set elements)
/// Handles peeking, comma writing, quotes, and error reporting
fn write_string_value<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    conv: &mut DdbConverter<'_, '_, W>,
    with_quotes: bool,
    write_comma_if_pending: bool,
    peek_context: &'static str,
    write_context: &'static str,
) -> StreamOp {
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            let position = rjiter.current_index();
            conv.store_rjiter_error(e, position, peek_context);
            return StreamOp::Error("Failed to peek string value");
        }
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string value");
    }

    if write_comma_if_pending {
        conv.write_comma_if_pending();
    }

    if with_quotes {
        conv.write(b"\"");
    }
    if let Err(e) = rjiter.write_long_bytes(conv.writer) {
        let position = rjiter.current_index();
        conv.store_rjiter_error(e, position, write_context);
        return StreamOp::Error("Failed to write value");
    }
    if with_quotes {
        conv.write(b"\"");
    }

    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

/// Handle a type key - for literal types, consume and write the value directly
fn on_type_key<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let type_key = match conv.current_field {
        Some(field) => field,
        None => return StreamOp::Error("current_field should be set for type key"),
    };

    // Helper for boolean-based types (BOOL/NULL): peek bool, consume with known_bool, write output
    fn handle_bool_based_type<R: embedded_io::Read, W: IoWrite>(
        rjiter: &mut RJiter<R>,
        conv: &mut DdbConverter<'_, '_, W>,
        validate_peek: impl Fn(Peek) -> Result<&'static [u8], &'static str>,
        type_name: &'static str,
    ) -> StreamOp {
        let peek = match rjiter.peek() {
            Ok(p) => p,
            Err(e) => {
                let position = rjiter.current_index();
                conv.store_rjiter_error(e, position, type_name);
                return StreamOp::Error("Failed to peek value");
            }
        };

        // Validate and get output bytes
        let output = match validate_peek(peek) {
            Ok(bytes) => bytes,
            Err(msg) => return StreamOp::Error(msg),
        };

        // Write comma if in L array
        if conv.l_depth > 0 {
            conv.write_comma_if_pending();
        }

        // Consume the value
        if let Err(e) = rjiter.known_bool(peek) {
            let position = rjiter.current_index();
            conv.store_rjiter_error(e, position, type_name);
            return StreamOp::Error("Failed to consume boolean value");
        }
        conv.write(output);

        conv.pending_comma = true;
        conv.current_type = None;
        StreamOp::ValueIsConsumed
    }

    match type_key {
        b"S" | b"B" => {
            let write_comma_if_pending = conv.l_depth > 0;
            let result = write_string_value(rjiter, &mut conv, true, write_comma_if_pending, "S/B (string) type", "S/B (string) type");
            conv.current_type = None;
            result
        }
        b"N" => {
            let write_comma_if_pending = conv.l_depth > 0;
            let result = write_string_value(rjiter, &mut conv, false, write_comma_if_pending, "N (number) type", "N (number) type");
            conv.current_type = None;
            result
        }
        b"BOOL" => handle_bool_based_type(
            rjiter, &mut conv,
            |peek| match peek {
                Peek::True => Ok(b"true"),
                Peek::False => Ok(b"false"),
                _ => Err("Expected boolean value for BOOL type"),
            },
            "BOOL type"
        ),
        b"NULL" => handle_bool_based_type(
            rjiter, &mut conv,
            |peek| match peek {
                Peek::True => Ok(b"null"),
                _ => Err("Expected true for NULL type"),
            },
            "NULL type"
        ),
        b"SS" | b"BS" => {
            // SS/BS type - write opening bracket here (parent handles it, not find_action_array)
            conv.write(b"[");
            conv.pending_comma = false;
            conv.current_type = Some(TypeDesc::SS);
            conv.phase = Phase::ExpectingValue;  // Stay in ExpectingValue, SS elements are atoms
            StreamOp::None
        }
        b"NS" => {
            // NS type - write opening bracket here (parent handles it, not find_action_array)
            conv.write(b"[");
            conv.pending_comma = false;
            conv.current_type = Some(TypeDesc::NS);
            conv.phase = Phase::ExpectingValue;  // Stay in ExpectingValue, NS elements are atoms
            StreamOp::None
        }
        b"L" => {
            // L type - write opening bracket here (parent handles it, not find_action_array)
            // Write comma if in L array (nested L)
            if conv.l_depth > 0 {
                conv.write_comma_if_pending();
            }
            conv.write(b"[");
            conv.pending_comma = false;
            conv.l_depth += 1;  // Track L nesting
            conv.current_type = Some(TypeDesc::L);
            conv.phase = Phase::ExpectingTypeKey;  // In L, we expect type keys (type descriptors are ignored)
            StreamOp::None
        }
        b"M" => {
            // M type - write opening brace here (parent handles it, not find_action_object)
            // Write comma if in L array
            if conv.l_depth > 0 {
                conv.write_comma_if_pending();
            }
            conv.write(b"{");
            conv.newline();
            conv.depth += 1;
            conv.pending_comma = false;
            conv.m_depth += 1;  // Track M nesting
            conv.current_type = Some(TypeDesc::M);
            conv.phase = Phase::ExpectingField;
            StreamOp::None
        }
        _ => {
            conv.store_parse_error(
                0,
                "Invalid DynamoDB JSON format: unknown type descriptor",
                Some(type_key),
            );
            StreamOp::Error("Unknown type descriptor")
        }
    }
}

// Type descriptor value handlers for set element atoms (SS, NS)

fn on_set_string_element<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    write_string_value(
        rjiter,
        &mut conv,
        true,  // with_quotes
        true,  // write_comma_if_pending: always for set elements (pending_comma handles first element)
        "peeking SS/BS (string set) element",
        "writing SS/BS (string set) element",
    )
}

fn on_set_number_element<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    write_string_value(
        rjiter,
        &mut conv,
        false,  // with_quotes
        true,   // write_comma_if_pending: always for set elements (pending_comma handles first element)
        "peeking NS (number set) element",
        "writing NS (number set) element",
    )
}

// Generic error handler that returns the stored error
fn on_error<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, _baton: DdbBaton<'_, '_, W>) -> StreamOp {
    StreamOp::Error("Validation error (see stored error)")
}

/// Handle Object structural pseudoname
fn find_action_object<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    _context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
    current_type: Option<TypeDesc>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Check if this is the root object (depth == 0)
    if baton.borrow().depth == 0 {
        return Some(on_root_object_begin);
    }

    // Validate context: only allow objects in valid contexts
    match phase {
        Phase::ExpectingValue => {
            // In ExpectingValue, only M type expects objects; all others (SS, NS, L) expect arrays
            // If we're here with an object, it's invalid
            let mut conv = baton.borrow_mut();
            conv.store_parse_error(
                0,
                "Invalid DynamoDB JSON format: unexpected object value",
                None,
            );
            Some(on_error)
        }
        Phase::ExpectingTypeKey => {
            // Check if L type is expecting an array but got an object
            if current_type == Some(TypeDesc::L) {
                let mut conv = baton.borrow_mut();
                conv.store_parse_error(
                    0,
                    "Invalid DynamoDB JSON format: L type expects an array value, not an object",
                    None,
                );
                return Some(on_error);
            }
            // Type descriptor objects are allowed in other contexts
            None
        }
        Phase::ExpectingField => {
            // M type context - objects (nested M values) are allowed
            None
        }
    }
}

fn find_action_key<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    mut context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    let key = context.next()?;

    let action: Option<Action<DdbBaton<'a, 'workbuf, W>, R>> = match phase {
        Phase::ExpectingField => {
            // Check for Item at top with AsWrapper - early return without side effects
            if key == b"Item" {
                let mode = baton.borrow().item_wrapper_mode;
                if mode == ItemWrapperMode::AsWrapper {
                    if let Some(b"#top") = context.next() {
                        return None;
                    }
                }
            }
            Some(on_field_key)
        }
        Phase::ExpectingTypeKey => Some(on_type_key),
        _ => {
            // Unexpected phase - set internal error
            let mut conv = baton.borrow_mut();
            conv.last_error = Some(ConversionError::ParseError {
                position: 0,
                context: "Unexpected key in phase",
                unknown_type: None,
            });
            None
        }
    };

    // Store the key if we have an action to execute
    if action.is_some() {
        let mut conv = baton.borrow_mut();
        #[allow(unsafe_code)]
        let key_slice: &'workbuf [u8] =
            unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(key) };
        conv.current_field = Some(key_slice);
    }

    action
}

/// Handle Array structural pseudoname
fn find_action_array<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    _context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    _phase: Phase,
    current_type: Option<TypeDesc>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Validate context: only allow arrays for SS, NS, L types
    match current_type {
        Some(TypeDesc::SS) | Some(TypeDesc::NS) => {
            // Valid: these types expect arrays
            None
        }
        Some(TypeDesc::L) => {
            // Valid: L expects array. Clear current_type so elements inside don't inherit it
            let mut conv = baton.borrow_mut();
            conv.current_type = None;
            None
        }
        _ => {
            // All other cases: arrays are not valid
            let mut conv = baton.borrow_mut();
            conv.store_parse_error(
                0,
                "Invalid DynamoDB JSON format: unexpected array value",
                None,
            );
            Some(on_error)
        }
    }
}

/// Handle Atom structural pseudoname
fn find_action_atom<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    mut context: ContextIter,
    _baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
    current_type: Option<TypeDesc>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Atoms are only valid as set elements inside arrays
    if phase != Phase::ExpectingValue {
        return Some(on_unexpected_atom);
    }

    // SS/NS set element inside array
    if matches!(current_type, Some(TypeDesc::SS) | Some(TypeDesc::NS)) {
        if let Some(first) = context.next() {
            if first == b"#array" {
                // Valid set element inside array
                return if current_type == Some(TypeDesc::SS) {
                    Some(on_set_string_element)
                } else {
                    Some(on_set_number_element)
                };
            }
        }
    }

    // All other cases: atoms are unexpected
    Some(on_unexpected_atom)
}

fn find_action<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
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

fn on_unexpected_atom<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.store_parse_error(
        0,
        "Invalid DynamoDB JSON format: Expected array for set type, atom values only allowed as set elements",
        None,
    );
    StreamOp::Error("Expected array for set type, atom values only allowed as set elements")
}

fn on_list_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.write(b"]");
    conv.pending_comma = true;

    // Decrement L depth
    conv.l_depth = conv.l_depth.saturating_sub(1);

    // Ending L array - restore phase based on whether we're still in another L
    conv.phase = if conv.l_depth > 0 { Phase::ExpectingTypeKey } else { Phase::ExpectingField };
    conv.current_type = None;
    Ok(())
}

fn on_set_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.write(b"]");
    conv.pending_comma = true;

    // Ending SS/NS set - restore phase based on whether we're in an L array
    conv.current_type = None;
    conv.phase = if conv.l_depth > 0 { Phase::ExpectingTypeKey } else { Phase::ExpectingField };
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
    conv.phase = if conv.l_depth > 0 { Phase::ExpectingTypeKey } else { Phase::ExpectingField };
    Ok(())
}

fn on_type_key_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Type key value ended (for literal types: S, N, B, BOOL, NULL)
    // Restore phase based on context
    let mut conv = baton.borrow_mut();
    // Priority: M object (if we're in one, expect another field)
    // Otherwise: L array (expect another type descriptor)
    // Otherwise: root (expect a field)
    conv.phase = if conv.m_depth > 0 {
        Phase::ExpectingField
    } else if conv.l_depth > 0 {
        Phase::ExpectingTypeKey
    } else {
        Phase::ExpectingField
    };
    Ok(())
}

fn on_root_object_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let conv = baton.borrow();
    // If we opened a root object (depth > 0), close it
    // Both Item wrapper and no-Item cases write the same braces
    if conv.depth > 0 {
        drop(conv);
        on_item_end(baton)?;
    }
    Ok(())
}

/// Handle Object structural pseudoname for end actions
fn find_end_action_object<'a, 'workbuf, W: IoWrite>(
    context: ContextIter,
    _baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<EndAction<DdbBaton<'a, 'workbuf, W>>> {
    // Check if this is the root object ending (#top in context)
    let mut ctx = context;
    if let Some(key) = ctx.next() {
        if key == b"#top" {
            return Some(on_root_object_end);
        }
    }
    None
}

/// Handle end-actions for keys - this is where all end-action logic resides
fn find_end_action_key<'a, 'workbuf, W: IoWrite>(
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<EndAction<DdbBaton<'a, 'workbuf, W>>> {
    let mut ctx = context.clone();
    let key = ctx.next()?;

    match key {
        b"L" => {
            // Only call on_list_end if this is actually an L array type descriptor
            // Check if the parent suggests this is a field name inside M
            if let Some(parent) = ctx.next() {
                if parent == b"M" {
                    // "L" is a field name inside M object, not an L array
                    Some(on_type_key_end)
                } else {
                    // "L" is an L array type descriptor
                    Some(on_list_end)
                }
            } else {
                Some(on_list_end)
            }
        }
        b"SS" | b"NS" | b"BS" => {
            // Check if this is a set type descriptor or a field name
            if let Some(parent) = ctx.next() {
                if parent == b"M" {
                    // Field name inside M object
                    Some(on_type_key_end)
                } else {
                    // Set type descriptor
                    Some(on_set_end)
                }
            } else {
                Some(on_set_end)
            }
        }
        b"M" => {
            // Could be M object value or type descriptor
            let m_depth = baton.borrow().m_depth;
            if m_depth > 0 && ending_m_object_from_context(context) {
                Some(on_map_end)
            } else {
                None
            }
        }
        b"Item" => {
            // Check if this is the Item wrapper (parent is #top) or a field named "Item"
            if let Some(parent) = ctx.next() {
                if parent == b"#top" {
                    // At top level - check item_wrapper_mode
                    let mode = baton.borrow().item_wrapper_mode;
                    if mode == ItemWrapperMode::AsWrapper {
                        None  // Transparent - Item wrapper has no end action
                    } else {
                        // AsField mode - treat as a regular field
                        Some(on_type_key_end)
                    }
                } else {
                    // Field named "Item" inside M - type descriptor ending
                    Some(on_type_key_end)
                }
            } else {
                None
            }
        }
        _ => {
            // Any other key - this is a type descriptor object ending
            // Literal types (S, N, BOOL, NULL, B) need phase restoration
            Some(on_type_key_end)
        }
    }
}

fn find_end_action<'a, 'workbuf, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<EndAction<DdbBaton<'a, 'workbuf, W>>> {
    match structural {
        StructuralPseudoname::Array => None,
        StructuralPseudoname::Object => find_end_action_object(context, baton),
        StructuralPseudoname::None => find_end_action_key(context, baton),
        StructuralPseudoname::Atom => None,
    }
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
/// * `item_wrapper_mode` - How to handle "Item" key at top level (AsWrapper or AsField)
///
/// # Returns
/// `Ok(())` on success, or `Err(ConversionError)` with detailed error information on failure
pub fn convert_ddb_to_normal<R: IoRead, W: IoWrite>(
    reader: &mut R,
    writer: &mut W,
    rjiter_buffer: &mut [u8],
    context_buffer: &mut [u8],
    pretty: bool,
    item_wrapper_mode: ItemWrapperMode,
) -> Result<(), ConversionError> {
    let mut rjiter = RJiter::new(reader, rjiter_buffer);

    let converter = DdbConverter::new(writer, pretty, item_wrapper_mode);
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
