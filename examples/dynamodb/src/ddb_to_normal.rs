use embedded_io::{Read as IoRead, Write as IoWrite};
use rjiter::jiter::Peek;
use rjiter::RJiter;
use scan_json::{scan, Action, EndAction, Options, StreamOp};
use scan_json::matcher::StructuralPseudoname;
use scan_json::stack::ContextIter;
use core::cell::RefCell;
use u8pool::U8Pool;
use crate::ConversionError;
use alloc::vec::Vec;

/// Parse mode representing what we're currently expecting
#[derive(Debug, Clone, Copy, PartialEq)]
enum ParseMode {
    Root,              // At root level, expecting "Item" key or field names
    FieldNames,        // Expecting field names (inside Item or at root without wrapper)
    TypeDescriptor,    // Inside type descriptor object, expecting type key
    InS,              // Inside S type descriptor, expecting string value
    InN,              // Inside N type descriptor, expecting string value (number)
    InBool,           // Inside BOOL type descriptor, expecting boolean value
    InNull,           // Inside NULL type descriptor, expecting true value
    ExpectSSArray,    // Expecting array for SS/BS type
    ExpectNSArray,    // Expecting array for NS type
    ExpectLArray,     // Expecting array for L type
    ExpectMObject,    // Expecting object for M type
    InSS,             // Inside SS/BS array, expecting string elements
    InNS,             // Inside NS array, expecting string (number) elements
    InL,              // Inside L array, expecting type descriptor elements
    InM,              // Inside M object, expecting field names
}

pub struct DdbConverter<'a, 'workbuf, W: IoWrite> {
    writer: &'a mut W,
    pending_comma: bool,
    pretty: bool,
    depth: usize,
    current_field: Option<&'workbuf [u8]>,
    has_item_wrapper: Option<bool>, // None = unknown, Some(true) = has Item, Some(false) = no Item
    last_error: Option<ConversionError>, // Stores detailed error information
    mode_stack: Vec<ParseMode>, // Stack of parse modes
    skip_next_object: bool, // Skip treating next object as type descriptor (for Item value)
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
            mode_stack: Vec::from([ParseMode::Root]),
            skip_next_object: false,
        }
    }

    fn current_mode(&self) -> ParseMode {
        *self.mode_stack.last().unwrap_or(&ParseMode::Root)
    }

    fn push_mode(&mut self, mode: ParseMode) {
        self.mode_stack.push(mode);
    }

    fn pop_mode(&mut self) {
        self.mode_stack.pop();
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

/// Handle the start of Item value object - write opening brace and enter FieldNames mode
fn on_item_value_object_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.write(b"{");
    conv.newline();
    conv.depth = 1;
    conv.pending_comma = false;
    conv.push_mode(ParseMode::FieldNames);
    StreamOp::None
}

/// Handle the end of Item object - write closing brace and newline for JSONL
fn on_item_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.write(b"}");
    conv.write(b"\n");
    conv.pop_mode(); // Pop FieldNames
    Ok(())
}

/// Handle a field key - write the field name and prepare for type descriptor
fn on_field_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let field_name = {
        let conv = baton.borrow();
        conv.current_field.expect("current_field should be set").to_vec()
    };

    let mut conv = baton.borrow_mut();

    // If we're in Root mode (no Item wrapper), write opening brace on first field
    if conv.current_mode() == ParseMode::Root {
        conv.write(b"{");
        conv.newline();
        conv.depth = 1;
        conv.has_item_wrapper = Some(false);
        conv.pop_mode(); // Pop Root
        conv.push_mode(ParseMode::FieldNames);
    }

    conv.write_comma();
    conv.indent();
    conv.write(b"\"");
    conv.write(&field_name);
    conv.write(b"\":");
    conv.pending_comma = false;

    StreamOp::None
}

/// Handle the start of a type descriptor object
fn on_type_descriptor_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();

    // In L arrays, write comma before element
    if conv.current_mode() == ParseMode::InL {
        conv.write_comma();
    }

    conv.push_mode(ParseMode::TypeDescriptor);
    StreamOp::None
}

/// Handle a type key and transition to appropriate mode
fn on_type_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let type_key = {
        let conv = baton.borrow();
        conv.current_field.expect("current_field should be set for type key").to_vec()
    };

    let mut conv = baton.borrow_mut();

    match type_key.as_slice() {
        b"S" | b"B" => conv.push_mode(ParseMode::InS),
        b"N" => conv.push_mode(ParseMode::InN),
        b"BOOL" => conv.push_mode(ParseMode::InBool),
        b"NULL" => conv.push_mode(ParseMode::InNull),
        b"SS" | b"BS" => conv.push_mode(ParseMode::ExpectSSArray),
        b"NS" => conv.push_mode(ParseMode::ExpectNSArray),
        b"L" => conv.push_mode(ParseMode::ExpectLArray),
        b"M" => conv.push_mode(ParseMode::ExpectMObject),
        _ => {
            conv.store_parse_error(
                0,
                "Invalid DynamoDB JSON format: unknown type descriptor",
                Some(&type_key),
            );
            return StreamOp::Error("Unknown type descriptor");
        }
    }

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
    conv.pop_mode(); // Pop InS
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
    conv.pop_mode(); // Pop InN
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
    conv.pop_mode(); // Pop InBool
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
    conv.pop_mode(); // Pop InNull
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
    conv.pop_mode(); // Pop ExpectSSArray
    conv.push_mode(ParseMode::InSS);
    conv.write(b"[");
    conv.pending_comma = false;
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
    conv.pop_mode(); // Pop ExpectNSArray
    conv.push_mode(ParseMode::InNS);
    conv.write(b"[");
    conv.pending_comma = false;
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
    conv.pop_mode(); // Pop ExpectLArray
    conv.push_mode(ParseMode::InL);
    conv.write(b"[");
    conv.pending_comma = false;
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
    conv.pop_mode(); // Pop ExpectMObject
    conv.push_mode(ParseMode::InM);
    conv.write(b"{");
    conv.newline();
    conv.depth += 1;
    conv.pending_comma = false;
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

fn find_action<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    let mode = baton.borrow().current_mode();

    // Match the root object - special case
    if structural == StructuralPseudoname::Object && mode == ParseMode::Root {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            if first == b"#top" {
                // This is the root object - don't treat it as a type descriptor
                return Some(on_root_object_begin);
            }
        }
    }

    // Match keys (field names or type keys)
    if structural == StructuralPseudoname::None {
        let mut ctx = context.clone();
        if let Some(key) = ctx.next() {
            // Check if this is "Item" key at root level
            if key == b"Item" && mode == ParseMode::Root {
                // Verify parent is #top
                if let Some(parent) = ctx.next() {
                    if parent == b"#top" {
                        let mut conv = baton.borrow_mut();
                        #[allow(unsafe_code)]
                        let key_slice: &'workbuf [u8] =
                            unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(key) };
                        conv.current_field = Some(key_slice);
                        drop(conv);
                        return Some(on_item_key);
                    }
                }
            }

            // Store the key
            let mut conv = baton.borrow_mut();
            #[allow(unsafe_code)]
            let key_slice: &'workbuf [u8] =
                unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(key) };
            conv.current_field = Some(key_slice);
            drop(conv);

            match mode {
                ParseMode::Root | ParseMode::FieldNames | ParseMode::InM => {
                    // Regular field name
                    return Some(on_field_key);
                }
                ParseMode::TypeDescriptor => {
                    // Type key inside type descriptor
                    return Some(on_type_key);
                }
                _ => {}
            }
        }
    }

    // Match objects
    if structural == StructuralPseudoname::Object {
        // Check if this is the Item value object
        let mut conv = baton.borrow_mut();
        if conv.skip_next_object {
            conv.skip_next_object = false;
            drop(conv);
            return Some(on_item_value_object_begin);
        }
        drop(conv);

        match mode {
            ParseMode::FieldNames | ParseMode::InM | ParseMode::InL => {
                // Type descriptor object
                return Some(on_type_descriptor_begin);
            }
            _ => {}
        }
    }

    // Match arrays
    if structural == StructuralPseudoname::Array {
        match mode {
            ParseMode::ExpectSSArray => return Some(on_string_set_begin),
            ParseMode::ExpectNSArray => return Some(on_number_set_begin),
            ParseMode::ExpectLArray => return Some(on_list_begin),
            // Validation: if we're expecting an object but got an array, it's an error
            ParseMode::ExpectMObject => {
                return Some(on_invalid_type_value_not_object);
            }
            // Validation: if we're expecting an atom but got an array, it's an error
            ParseMode::InS | ParseMode::InN | ParseMode::InBool | ParseMode::InNull => {
                return Some(on_invalid_type_value_not_atom);
            }
            _ => {}
        }
    }

    // Match objects for M type
    if structural == StructuralPseudoname::Object {
        // Skip the check from earlier since we're checking mode again
        if mode == ParseMode::ExpectMObject {
            return Some(on_map_begin);
        }
        // Validation: if we're expecting an array but got an object, it's an error
        match mode {
            ParseMode::ExpectSSArray | ParseMode::ExpectNSArray | ParseMode::ExpectLArray => {
                return Some(on_invalid_type_value_not_array);
            }
            // Validation: if we're expecting an atom but got an object, it's an error
            ParseMode::InS | ParseMode::InN | ParseMode::InBool | ParseMode::InNull => {
                return Some(on_invalid_type_value_not_atom);
            }
            _ => {}
        }
    }

    // Match atom values
    if structural == StructuralPseudoname::Atom {
        match mode {
            ParseMode::InS => return Some(on_string_value),
            ParseMode::InN => return Some(on_number_value),
            ParseMode::InBool => return Some(on_bool_value),
            ParseMode::InNull => return Some(on_null_value),
            ParseMode::InSS => return Some(on_set_string_element),
            ParseMode::InNS => return Some(on_set_number_element),
            // Validation: if we're expecting an array but got an atom, it's an error
            ParseMode::ExpectSSArray | ParseMode::ExpectNSArray | ParseMode::ExpectLArray => {
                return Some(on_invalid_type_value_not_array);
            }
            // Validation: if we're expecting an object but got an atom, it's an error
            ParseMode::ExpectMObject => {
                return Some(on_invalid_type_value_not_object);
            }
            _ => {}
        }
    }

    None
}

fn on_invalid_type_value_not_array<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let mode = conv.current_mode();
    let type_name = match mode {
        ParseMode::ExpectSSArray => "SS/BS",
        ParseMode::ExpectNSArray => "NS",
        ParseMode::ExpectLArray => "L",
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
    let mode = conv.current_mode();
    let type_name = match mode {
        ParseMode::InS => "S",
        ParseMode::InN => "N",
        ParseMode::InBool => "BOOL",
        ParseMode::InNull => "NULL",
        _ => "unknown",
    };
    conv.store_parse_error(
        0,
        "Invalid DynamoDB JSON format: type expects a primitive value",
        Some(type_name.as_bytes()),
    );
    StreamOp::Error("Expected primitive value for type")
}

fn on_set_or_list_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.write(b"]");
    conv.pending_comma = true;
    conv.pop_mode(); // Pop InSS/InNS/InL
    Ok(())
}

fn on_map_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;
    conv.pop_mode(); // Pop InM
    Ok(())
}

fn on_type_descriptor_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.pop_mode(); // Pop TypeDescriptor
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
    let mode = baton.borrow().current_mode();

    // Match end of root object
    if structural == StructuralPseudoname::Object && mode == ParseMode::Root {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            if first == b"#top" {
                return Some(on_root_object_end);
            }
        }
    }

    // Match end of objects
    if structural == StructuralPseudoname::Object {
        match mode {
            ParseMode::FieldNames => {
                // Check if this is the Item/root object
                let has_wrapper = baton.borrow().has_item_wrapper;
                if has_wrapper.is_some() {
                    return Some(on_item_end);
                }
            }
            ParseMode::InM => {
                return Some(on_map_end);
            }
            ParseMode::TypeDescriptor => {
                return Some(on_type_descriptor_end);
            }
            _ => {}
        }
    }

    // Match end of arrays
    if structural == StructuralPseudoname::Array {
        match mode {
            ParseMode::InSS | ParseMode::InNS | ParseMode::InL => {
                return Some(on_set_or_list_end);
            }
            _ => {}
        }
    }

    // Match end of type descriptor objects in L arrays
    if structural == StructuralPseudoname::Object {
        if mode == ParseMode::TypeDescriptor {
            // Check if parent is InL
            let mode_stack = &baton.borrow().mode_stack;
            if mode_stack.len() >= 2 && mode_stack[mode_stack.len() - 2] == ParseMode::InL {
                return Some(on_list_element_end);
            }
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
