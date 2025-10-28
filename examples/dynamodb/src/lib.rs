#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use embedded_io::{Read as IoRead, Write as IoWrite};
use rjiter::jiter::Peek;
use rjiter::RJiter;
use scan_json::{iter_match, scan, Action, EndAction, Options, StreamOp};
use scan_json::matcher::StructuralPseudoname;
use scan_json::stack::ContextIter;
use core::cell::RefCell;
use u8pool::U8Pool;

/// Detailed error information for conversion errors
#[derive(Debug, Clone)]
pub enum ConversionError {
    /// RJiter error with context
    RJiterError {
        kind: rjiter::error::ErrorType,
        position: usize,
        context: &'static str,
    },
    /// IO error with context
    IOError {
        kind: embedded_io::ErrorKind,
        position: usize,
        context: &'static str,
    },
    /// Parse error (invalid DynamoDB JSON format)
    ParseError {
        position: usize,
        context: &'static str,
        /// Unknown type descriptor bytes (buffer, actual length used)
        unknown_type: Option<([u8; 32], usize)>,
    },
    /// Scan error (from scan_json library)
    ScanError(scan_json::Error),
}

#[cfg(feature = "std")]
impl core::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConversionError::RJiterError { kind, position, context } => {
                write!(f, "RJiter error at position {}: {:?} (while {})", position, kind, context)
            }
            ConversionError::IOError { kind, position, context } => {
                write!(f, "IO error at position {}: {:?} (while {})", position, kind, context)
            }
            ConversionError::ParseError { position, context, unknown_type } => {
                if let Some((bytes, len)) = unknown_type {
                    let type_str = std::string::String::from_utf8_lossy(&bytes[..*len]);
                    write!(f, "Parse error at position {}: {} (unknown type descriptor '{}')", position, context, type_str)
                } else {
                    write!(f, "Parse error at position {}: {}", position, context)
                }
            }
            ConversionError::ScanError(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

pub struct DdbConverter<'a, 'workbuf, W: IoWrite> {
    writer: &'a mut W,
    pending_comma: bool,
    pretty: bool,
    depth: usize,
    current_field: Option<&'workbuf [u8]>,
    has_item_wrapper: Option<bool>, // None = unknown, Some(true) = has Item, Some(false) = no Item
    last_error: Option<ConversionError>, // Stores detailed error information
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

/// Handle the start of Item object - write opening brace
fn on_item_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.write(b"{");
    conv.newline();
    conv.depth = 1;
    conv.pending_comma = false;
    conv.has_item_wrapper = Some(true);
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

/// Handle a field key inside Item - write the field name
fn on_item_field_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let field_name = {
        let conv = baton.borrow();
        conv.current_field.expect("current_field should be set").to_vec()
    };

    let mut conv = baton.borrow_mut();

    // If we haven't written anything yet and there's no Item wrapper, write opening brace
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

    StreamOp::None
}

// Type descriptor value handlers - these only use peek and write_long_bytes

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
    StreamOp::ValueIsConsumed  // Tell scan we consumed the value
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
    StreamOp::ValueIsConsumed  // Tell scan we consumed the value
}

fn on_bool_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    // Peek to see if it's true or false
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
    StreamOp::ValueIsConsumed
}

fn on_null_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    // NULL value in DDB is {"NULL": true}
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
    StreamOp::ValueIsConsumed
}

fn on_string_set_begin<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Validate that the value is actually an array
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
    StreamOp::None
}

fn on_number_set_begin<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Validate that the value is actually an array
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
    StreamOp::None
}

fn on_list_begin<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Validate that the value is actually an array
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
    StreamOp::None
}

fn on_map_begin<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Validate that the value is actually an object
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
    StreamOp::None
}

fn on_type_descriptor_object<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Type descriptor object - just let scan handle it
    // But if we're inside an L array, write comma before this element
    let mut conv = baton.borrow_mut();
    conv.write_comma();
    StreamOp::None
}

fn on_set_string_element<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let position = rjiter.current_index();
    // String element in SS/BS set - write with quotes and comma
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
    // Number element in NS set - write without quotes but with comma
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

fn on_ddb_format_error<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, _baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Error was already stored in last_error when the format error was detected
    StreamOp::Error("DynamoDB format validation error")
}

/// Helper to check if we're in a context where type descriptors appear
/// Type descriptors can appear under:
/// - Item (top level with wrapper)
/// - #top (top level without Item wrapper)
/// - M (Map object values)
/// - L (List array elements)
fn is_type_descriptor_context(mut ctx: ContextIter) -> bool {
    // Walk up the context to find if we're under Item, M, L, or directly at #top
    let first = ctx.next();
    if first == Some(b"#top") {
        // Directly at top level (no Item wrapper)
        return true;
    }

    // Continue walking for Item, M, or L
    let mut current = first;
    loop {
        match current {
            Some(b"Item") => return true,
            Some(b"M") => return true,
            Some(b"L") => return true,
            Some(b"#top") | None => return false,
            Some(_) => {
                current = ctx.next();
            }
        }
    }
}

/// Helper to count consecutive occurrences of a specific key in the context
/// Returns the count of consecutive matches starting from the current position
fn count_consecutive(mut ctx: ContextIter, key: &[u8]) -> usize {
    let mut count = 0;
    while let Some(next) = ctx.next() {
        if next == key {
            count += 1;
        } else {
            break;
        }
    }
    count
}

/// Check if we're in a "rogue name" scenario where a field name matches a type descriptor
/// For repeating keys like [M, M, M, ...], odd count = real type, even = field name
fn is_real_type_context(first_key: &[u8], second_key: &[u8], ctx: ContextIter) -> bool {
    if first_key == second_key {
        // Repeating key - use parity: odd count = type, even = field
        let total_count = 2 + count_consecutive(ctx, first_key);
        total_count % 2 == 1
    } else {
        // Different keys - this is a real type descriptor
        true
    }
}

fn find_action<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Match "Item" key at root (optional wrapper)
    if iter_match(|| [b"Item", b"#top"], structural, context.clone()) {
        return Some(on_item_begin);
    }

    // Match field keys - can be inside Item, at top-level, or inside M type objects
    if structural == StructuralPseudoname::None {
        let mut ctx = context.clone();
        if let Some(field) = ctx.next() {
            if field != b"#top" && field != b"#array" {
                if let Some(parent) = ctx.next() {
                    // Field inside top-level Item
                    if parent == b"Item" {
                        // Check if this is really the top-level Item or a field named "Item"
                        // Top-level Item has context [..., Item, #top]
                        // Field named "Item" has context [..., Item, M, ...]
                        if let Some(grandparent) = ctx.next() {
                            if grandparent == b"#top" {
                                // This is a field inside the top-level Item
                                let mut conv = baton.borrow_mut();
                                #[allow(unsafe_code)]
                                let field_slice: &'workbuf [u8] =
                                    unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(field) };
                                conv.current_field = Some(field_slice);
                                return Some(on_item_field_key);
                            }
                            // Otherwise, this might be a type key under a field named "Item"
                            // Fall through to type descriptor matching
                        }
                    }
                    // Field at top-level (no Item wrapper)
                    else if parent == b"#top" {
                        let mut conv = baton.borrow_mut();
                        #[allow(unsafe_code)]
                        let field_slice: &'workbuf [u8] =
                            unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(field) };
                        conv.current_field = Some(field_slice);
                        return Some(on_item_field_key);
                    }
                    // Field inside M type object
                    else if parent == b"M" {
                        // Check for "rogue name" - is this inside a real M type or a field named "M"?
                        // Context is now [data, Item, ...] after parent, so check next element
                        let mut ctx_check = ctx.clone();
                        let grandparent = ctx_check.next().unwrap_or(b"");

                        if is_real_type_context(b"M", grandparent, ctx_check) {
                            let mut conv = baton.borrow_mut();
                            #[allow(unsafe_code)]
                            let field_slice: &'workbuf [u8] =
                                unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(field) };
                            conv.current_field = Some(field_slice);
                            return Some(on_item_field_key);
                        }
                    }
                }
            }
        }
    }

    // Match type descriptor objects - Objects that are values of fields or array elements
    if structural == StructuralPseudoname::Object {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            // Type descriptors under M or Item fields, or at top-level (no Item wrapper)
            if first != b"#top" && first != b"#array" {
                if let Some(parent) = ctx.next() {
                    if parent == b"Item" || parent == b"M" || parent == b"#top" {
                        return Some(on_type_descriptor_object);
                    }
                }
            }
            // Type descriptors inside L arrays: context is [#array, L, ...]
            else if first == b"#array" {
                if let Some(parent) = ctx.next() {
                    if parent == b"L" {
                        return Some(on_type_descriptor_object);
                    }
                }
            }
        }
    }

    // Match type descriptor keys (N, S, SS, M, L, etc.)
    if structural == StructuralPseudoname::None {
        let mut ctx = context.clone();
        if let Some(type_key) = ctx.next() {
            if let Some(second) = ctx.next() {
                // Type keys in normal fields: [typeKey, fieldName, Item/M, ...]
                // Type keys in L arrays: [typeKey, #array, L, ...]
                let in_type_descriptor_context = if second == b"#array" {
                    // Inside an array - check if it's an L array
                    if let Some(parent) = ctx.next() {
                        parent == b"L"
                    } else {
                        false
                    }
                } else if second != b"#top" {
                    // Normal field context
                    is_type_descriptor_context(ctx.clone())
                } else {
                    false
                };

                if in_type_descriptor_context {
                    // Match specific type keys and return handlers
                    match type_key {
                        b"S" | b"B" => return Some(on_string_value),
                        b"N" => return Some(on_number_value),
                        b"BOOL" => return Some(on_bool_value),
                        b"NULL" => return Some(on_null_value),
                        b"SS" | b"BS" => return Some(on_string_set_begin),
                        b"NS" => return Some(on_number_set_begin),
                        b"L" => return Some(on_list_begin),
                        b"M" => return Some(on_map_begin),
                        _ => {
                            // Unknown type descriptor - store error and return error handler
                            let mut conv = baton.borrow_mut();
                            conv.store_parse_error(
                                0, // Position not available here, will be set in handler
                                "Invalid DynamoDB JSON format: unknown type descriptor",
                                Some(type_key),
                            );
                            return Some(on_ddb_format_error);
                        }
                    }
                }
            }
        }
    }

    // Match array elements inside SS/BS/NS sets
    if structural == StructuralPseudoname::Atom {
        let mut ctx = context.clone();
        if let Some(_array_marker) = ctx.next() {  // #array
            if let Some(type_key) = ctx.next() {
                match type_key {
                    b"SS" | b"BS" => return Some(on_set_string_element),
                    b"NS" => return Some(on_set_number_element),
                    _ => {}
                }
            }
        }
    }

    None
}

fn on_set_or_list_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.write(b"]");
    conv.pending_comma = true;
    Ok(())
}

fn on_map_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;
    Ok(())
}

fn on_type_descriptor_end<W: IoWrite>(_baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Type descriptor object ended - nothing to write, value was already written
    // The value handlers already set pending_comma for the next element
    Ok(())
}

fn on_list_element_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Element in L array ended - prepare comma for next element
    let mut conv = baton.borrow_mut();
    conv.pending_comma = true;
    Ok(())
}

fn find_end_action<'a, 'workbuf, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    _baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<EndAction<DdbBaton<'a, 'workbuf, W>>> {
    // Match end of "Item"
    if iter_match(|| [b"Item", b"#top"], structural, context.clone()) {
        return Some(on_item_end);
    }

    // Match end of M objects (Map values inside type descriptors)
    if structural == StructuralPseudoname::Object {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            // Check if this is an M object ending
            if first == b"M" {
                if let Some(second) = ctx.next() {
                    // Check special cases: array context or regular field
                    let should_close = if second == b"#array" {
                        // M object inside an L array
                        ctx.next() == Some(b"L")
                    } else if second != b"#top" {
                        // Check if this is a real M type or a field named "M"
                        is_real_type_context(first, second, ctx.clone())
                    } else {
                        false
                    };

                    if should_close {
                        return Some(on_map_end);
                    }
                }
            }
            // Type descriptor objects inside L arrays
            if first == b"#array" {
                if let Some(parent) = ctx.next() {
                    if parent == b"L" {
                        // This is an element in L array - need to handle comma
                        return Some(on_list_element_end);
                    }
                }
            }
            // Match end of type descriptor objects (not inside arrays)
            if first == b"Item" {
                // Already handled above
            } else if first != b"#top" && first != b"#array" {
                // This is a type descriptor object (like {"S": "..."} or field named M/L)
                return Some(on_type_descriptor_end);
            }
        }
    }

    // Match end of SS, NS, BS, L arrays
    if structural == StructuralPseudoname::Array {
        let mut ctx = context.clone();
        if let Some(type_key) = ctx.next() {
            if let Some(_field_name) = ctx.next() {
                if is_type_descriptor_context(ctx) {
                    match type_key {
                        b"SS" | b"BS" | b"NS" | b"L" => return Some(on_set_or_list_end),
                        _ => {}
                    }
                }
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

    // If there was no Item wrapper, we need to close the root object
    let conv = baton.borrow();
    if conv.has_item_wrapper == Some(false) {
        drop(conv); // Release borrow before calling end handler
        on_item_end(&baton).map_err(|msg| ConversionError::ScanError(
            scan_json::Error::InternalError {
                position: rjiter.current_index(),
                message: msg,
            }
        ))?;
    }

    Ok(())
}

// ============================================================================
// Normal JSON to DynamoDB JSON conversion
// ============================================================================

pub struct NormalToDdbConverter<'a, 'workbuf, W: IoWrite> {
    writer: &'a mut W,
    pending_comma: bool,
    with_item_wrapper: bool,
    is_first_object: bool,
    current_field: Option<&'workbuf [u8]>,
    pretty: bool,
    depth: usize,
}

impl<'a, 'workbuf, W: IoWrite> NormalToDdbConverter<'a, 'workbuf, W> {
    fn new(writer: &'a mut W, with_item_wrapper: bool, pretty: bool) -> Self {
        Self {
            writer,
            pending_comma: false,
            with_item_wrapper,
            is_first_object: true,
            current_field: None,
            pretty,
            depth: 0,
        }
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

type NormalToDdbBaton<'a, 'workbuf, W> = &'a RefCell<NormalToDdbConverter<'a, 'workbuf, W>>;

fn on_root_object_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    if conv.with_item_wrapper {
        conv.write(b"{");
        conv.newline();
        conv.depth += 1;
        conv.indent();
        conv.write(b"\"Item\":{");
        conv.newline();
        conv.depth += 1;
    } else {
        conv.write(b"{");
        conv.newline();
        conv.depth += 1;
    }
    conv.pending_comma = false;
    conv.is_first_object = false;
    StreamOp::None
}

fn on_root_field_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    let field_name = {
        let conv = baton.borrow();
        conv.current_field.expect("current_field should be set").to_vec()
    };

    let mut conv = baton.borrow_mut();
    conv.write_comma();
    conv.indent();
    conv.write(b"\"");
    conv.write(&field_name);
    conv.write(b"\":{");
    conv.newline();
    conv.depth += 1;
    conv.pending_comma = false;
    StreamOp::None
}

fn on_nested_field_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    let field_name = {
        let conv = baton.borrow();
        conv.current_field.expect("current_field should be set").to_vec()
    };

    let mut conv = baton.borrow_mut();
    conv.write_comma();
    conv.indent();
    conv.write(b"\"");
    conv.write(&field_name);
    conv.write(b"\":{");
    conv.newline();
    conv.depth += 1;
    conv.pending_comma = false;
    StreamOp::None
}

fn on_string_value_toddb<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.indent();
    conv.write(b"\"S\":\"");
    let _ = rjiter.write_long_bytes(conv.writer);
    conv.write(b"\"");
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

fn on_bool_value_toddb<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(_) => return StreamOp::Error("Failed to peek boolean value"),
    };

    let mut conv = baton.borrow_mut();
    conv.indent();
    match peek {
        Peek::True => {
            let _ = rjiter.known_bool(peek);
            conv.write(b"\"BOOL\":true");
        }
        Peek::False => {
            let _ = rjiter.known_bool(peek);
            conv.write(b"\"BOOL\":false");
        }
        _ => return StreamOp::Error("Expected boolean value"),
    }
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

fn on_null_value_toddb<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(_) => return StreamOp::Error("Failed to peek null value"),
    };
    if peek != Peek::Null {
        return StreamOp::Error("Expected null value");
    }
    let _ = rjiter.known_null();

    let mut conv = baton.borrow_mut();
    conv.indent();
    conv.write(b"\"NULL\":true");
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

fn on_atom_value_toddb<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    // For atoms, we need to write {type:value} wrapper
    // The opening { is written here, closing } is written by type handlers
    {
        let mut conv = baton.borrow_mut();
        conv.write_comma();
        // Note: field handlers already wrote the opening {, so we don't write it for field values
        // But for array elements, we need it
    }

    // Peek to determine the actual type
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(_) => return StreamOp::Error("Failed to peek atom value"),
    };

    match peek {
        Peek::String => on_string_value_toddb(rjiter, baton),
        Peek::True | Peek::False => on_bool_value_toddb(rjiter, baton),
        Peek::Null => on_null_value_toddb(rjiter, baton),
        // Numbers: Int, Float, or any numeric peek type
        _ => {
            // Use next_number_bytes to preserve the exact string representation
            // This ensures "4.0" stays as "4.0" and doesn't become "4"
            let number_bytes = match rjiter.next_number_bytes() {
                Ok(bytes) => bytes,
                Err(_) => return StreamOp::Error("Failed to parse number"),
            };

            let mut conv = baton.borrow_mut();
            conv.indent();
            conv.write(b"\"N\":\"");
            conv.write(number_bytes);
            conv.write(b"\"");
            conv.newline();
            conv.depth -= 1;
            conv.indent();
            conv.write(b"}");
            conv.pending_comma = true;
            StreamOp::ValueIsConsumed
        }
    }
}

fn on_array_begin_toddb<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.indent();
    conv.write(b"\"L\":[");
    conv.newline();
    conv.pending_comma = false;
    StreamOp::None
}

fn on_array_element_atom<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    // Write opening brace for array element type wrapper, then handle the atom value
    {
        let mut conv = baton.borrow_mut();
        conv.write_comma();
        conv.write(b"{");
        conv.newline();
        conv.depth += 1;
        conv.pending_comma = false;
    }

    on_atom_value_toddb(rjiter, baton)
}

fn on_array_element_array<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    // Array inside array - write element wrapper and L type
    {
        let mut conv = baton.borrow_mut();
        conv.write_comma();
        conv.write(b"{");
        conv.newline();
        conv.depth += 1;
        conv.pending_comma = false;
    }

    on_array_begin_toddb(_rjiter, baton)
}

fn on_array_element_object<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    // Object inside array - write element wrapper and M type
    {
        let mut conv = baton.borrow_mut();
        conv.write_comma();
        conv.write(b"{");
        conv.newline();
        conv.depth += 1;
        conv.pending_comma = false;
    }

    on_nested_object_begin_toddb(_rjiter, baton)
}

fn on_nested_object_begin_toddb<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: NormalToDdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.indent();
    conv.write(b"\"M\":{");
    conv.newline();
    conv.depth += 1;
    conv.pending_comma = false;
    StreamOp::None
}

fn on_root_object_end<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    if conv.with_item_wrapper {
        conv.write(b"}");
        conv.newline();
        conv.depth -= 1;
        conv.indent();
        conv.write(b"}");
    } else {
        conv.write(b"}");
    }
    conv.write(b"\n");
    Ok(())
}

fn on_nested_object_end<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;
    Ok(())
}

fn on_array_end_toddb<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.indent();
    conv.write(b"]");
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;
    Ok(())
}

fn on_array_element_end_toddb<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Close the element wrapper with } (for atoms only) - note that the value handler already closed and decreased depth
    let mut conv = baton.borrow_mut();
    // Value handler already wrote the closing } and decreased depth
    // Nothing more to do here
    Ok(())
}

fn on_object_in_array_end<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Close the M object with } and then close the element wrapper with }
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;
    Ok(())
}

fn on_array_in_array_end<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Close the L array with ] and the element wrapper with }
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.indent();
    conv.write(b"]");
    conv.newline();
    conv.depth -= 1;
    conv.indent();
    conv.write(b"}");
    conv.pending_comma = true;
    Ok(())
}

fn find_action_toddb<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: NormalToDdbBaton<'a, 'workbuf, W>,
) -> Option<Action<NormalToDdbBaton<'a, 'workbuf, W>, R>> {
    // Match root object
    if structural == StructuralPseudoname::Object {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            if first == b"#top" {
                // This is the root object
                return Some(on_root_object_begin);
            }
        }
    }

    // Match field keys
    if structural == StructuralPseudoname::None {
        let mut ctx = context.clone();
        if let Some(field) = ctx.next() {
            // Store the field name
            let mut conv = baton.borrow_mut();
            #[allow(unsafe_code)]
            let field_slice: &'workbuf [u8] =
                unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(field) };
            conv.current_field = Some(field_slice);

            if let Some(parent) = ctx.next() {
                if parent == b"#top" {
                    // Root-level field
                    return Some(on_root_field_key);
                } else {
                    // Nested field (inside any object)
                    return Some(on_nested_field_key);
                }
            }
        }
    }

    // Check if we're in an array context first
    let mut ctx_check = context.clone();
    let in_array = if let Some(first) = ctx_check.next() {
        first == b"#array"
    } else {
        false
    };

    // Match values at root or nested level - Atom represents all primitives
    if structural == StructuralPseudoname::Atom {
        if in_array {
            // Array element - write opening brace first
            return Some(on_array_element_atom);
        }
        // Return a handler that will determine the type based on peek
        return Some(on_atom_value_toddb);
    }

    // Match arrays (convert to L type)
    if structural == StructuralPseudoname::Array {
        if in_array {
            // Nested array - write element wrapper
            return Some(on_array_element_array);
        }
        // Field value that's an array
        return Some(on_array_begin_toddb);
    }

    // Match nested objects (convert to M type)
    if structural == StructuralPseudoname::Object {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            if first == b"#top" {
                // Root object
                return None;
            } else if first == b"#array" {
                // Object in array - write element wrapper
                return Some(on_array_element_object);
            } else {
                // Object as a field value - write M type wrapper
                return Some(on_nested_object_begin_toddb);
            }
        }
    }

    None
}

fn find_end_action_toddb<'a, 'workbuf, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    _baton: NormalToDdbBaton<'a, 'workbuf, W>,
) -> Option<EndAction<NormalToDdbBaton<'a, 'workbuf, W>>> {
    // Match end of root object
    if structural == StructuralPseudoname::Object {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            if first == b"#top" {
                return Some(on_root_object_end);
            }
            // Objects in arrays - need to close both object and element wrapper
            if first == b"#array" {
                return Some(on_object_in_array_end);
            }
            // Nested objects (not in arrays)
            if first != b"#array" && first != b"#top" {
                // Any object that's not at the root and not in an array is a nested object
                return Some(on_nested_object_end);
            }
        }
    }

    // Match end of arrays
    if structural == StructuralPseudoname::Array {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            if first == b"#array" {
                // Array inside another array - close with ]} and element wrapper }
                return Some(on_array_in_array_end);
            } else {
                // Root-level or field value array - close with ]}
                return Some(on_array_end_toddb);
            }
        }
    }

    // Match end of array elements (primitives)
    if structural == StructuralPseudoname::Atom {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            if first == b"#array" {
                if let Some(parent) = ctx.next() {
                    if parent == b"L" {
                        return Some(on_array_element_end_toddb);
                    }
                }
            }
        }
    }

    None
}

/// Convert normal JSON to DynamoDB JSON in a streaming manner.
/// Supports JSONL format (newline-delimited JSON) - processes multiple JSON objects.
///
/// # Arguments
/// * `reader` - Input stream implementing `embedded_io::Read`
/// * `writer` - Output stream implementing `embedded_io::Write`
/// * `rjiter_buffer` - Buffer for rjiter to use (recommended: 4096 bytes)
/// * `context_buffer` - Buffer for scan_json context tracking (recommended: 2048 bytes)
/// * `pretty` - Whether to pretty-print the output (currently unused, may be added later)
/// * `with_item_wrapper` - Whether to wrap the output in an "Item" key
///
/// # Returns
/// `Ok(())` on success, or `Err(ConversionError)` with detailed error information on failure
pub fn convert_normal_to_ddb<R: IoRead, W: IoWrite>(
    reader: &mut R,
    writer: &mut W,
    rjiter_buffer: &mut [u8],
    context_buffer: &mut [u8],
    pretty: bool,
    with_item_wrapper: bool,
) -> Result<(), ConversionError> {
    let mut rjiter = RJiter::new(reader, rjiter_buffer);

    let converter = NormalToDdbConverter::new(writer, with_item_wrapper, pretty);
    let baton = RefCell::new(converter);

    let mut context = U8Pool::new(context_buffer, 32)
        .map_err(|_| ConversionError::ScanError(
            scan_json::Error::InternalError {
                position: 0,
                message: "Failed to create context pool"
            }
        ))?;

    scan(
        find_action_toddb,
        find_end_action_toddb,
        &mut rjiter,
        &baton,
        &mut context,
        &Options::new(),
    ).map_err(ConversionError::ScanError)?;

    Ok(())
}
