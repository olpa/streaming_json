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

pub struct DdbConverter<'a, 'workbuf, W: IoWrite> {
    writer: &'a mut W,
    pending_comma: bool,
    pretty: bool,
    depth: usize,
    current_field: Option<&'workbuf [u8]>,
}

impl<'a, 'workbuf, W: IoWrite> DdbConverter<'a, 'workbuf, W> {
    fn new(writer: &'a mut W, pretty: bool) -> Self {
        Self {
            writer,
            pending_comma: false,
            pretty,
            depth: 0,
            current_field: None,
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

type DdbBaton<'a, 'workbuf, W> = &'a RefCell<DdbConverter<'a, 'workbuf, W>>;

/// Handle the start of Item object - write opening brace
fn on_item_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.write(b"{");
    conv.newline();
    conv.depth = 1;
    conv.pending_comma = false;
    StreamOp::None
}

/// Handle the end of Item object - write closing brace
fn on_item_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline();
    conv.write(b"}");
    Ok(())
}

/// Handle a field key inside Item - write the field name
fn on_item_field_key<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let field_name = {
        let conv = baton.borrow();
        conv.current_field.expect("current_field should be set").to_vec()
    };

    let mut conv = baton.borrow_mut();
    conv.write_comma();
    conv.indent();
    conv.write(b"\"");
    conv.write(&field_name);
    conv.write(b"\": ");
    conv.pending_comma = false;

    StreamOp::None
}

// Type descriptor value handlers - these only use peek and write_long_bytes

fn on_string_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(_) => return StreamOp::Error("Failed to peek string"),
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string value");
    }

    let mut conv = baton.borrow_mut();
    conv.write(b"\"");
    if let Err(_) = rjiter.write_long_bytes(conv.writer) {
        return StreamOp::Error("Failed to write string");
    }
    conv.write(b"\"");
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed  // Tell scan we consumed the value
}

fn on_number_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(_) => return StreamOp::Error("Failed to peek number"),
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string value for number");
    }

    let mut conv = baton.borrow_mut();
    if let Err(_) = rjiter.write_long_bytes(conv.writer) {
        return StreamOp::Error("Failed to write number");
    }
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed  // Tell scan we consumed the value
}

fn on_bool_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Peek to see if it's true or false
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(_) => return StreamOp::Error("Failed to peek bool"),
    };

    let mut conv = baton.borrow_mut();
    match peek {
        Peek::True => {
            if rjiter.known_bool(peek).is_err() {
                return StreamOp::Error("Failed to consume true");
            }
            conv.write(b"true");
        }
        Peek::False => {
            if rjiter.known_bool(peek).is_err() {
                return StreamOp::Error("Failed to consume false");
            }
            conv.write(b"false");
        }
        _ => return StreamOp::Error("Expected boolean value"),
    }
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

fn on_null_value<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // NULL value in DDB is {"NULL": true}
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(_) => return StreamOp::Error("Failed to peek null"),
    };
    if peek != Peek::True {
        return StreamOp::Error("Expected true for NULL");
    }
    if rjiter.known_bool(peek).is_err() {
        return StreamOp::Error("Failed to consume null");
    }

    let mut conv = baton.borrow_mut();
    conv.write(b"null");
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

fn on_string_set_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.write(b"[");
    conv.pending_comma = false;
    StreamOp::None
}

fn on_number_set_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.write(b"[");
    conv.pending_comma = false;
    StreamOp::None
}

fn on_list_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.write(b"[");
    conv.pending_comma = false;
    StreamOp::None
}

fn on_map_begin<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.write(b"{");
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
    // String element in SS/BS set - write with quotes and comma
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(_) => return StreamOp::Error("Failed to peek set string"),
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string in set");
    }

    let mut conv = baton.borrow_mut();
    conv.write_comma();
    conv.write(b"\"");
    if let Err(_) = rjiter.write_long_bytes(conv.writer) {
        return StreamOp::Error("Failed to write set string");
    }
    conv.write(b"\"");
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

fn on_set_number_element<R: embedded_io::Read, W: IoWrite>(rjiter: &mut RJiter<R>, baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Number element in NS set - write without quotes but with comma
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(_) => return StreamOp::Error("Failed to peek set number"),
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string (number) in set");
    }

    let mut conv = baton.borrow_mut();
    conv.write_comma();
    if let Err(_) = rjiter.write_long_bytes(conv.writer) {
        return StreamOp::Error("Failed to write set number");
    }
    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

/// Helper to check if we're in a context where type descriptors appear
/// Type descriptors can appear under:
/// - Item (top level)
/// - M (Map object values)
/// - L (List array elements)
fn is_type_descriptor_context(mut ctx: ContextIter) -> bool {
    // Walk up the context to find if we're under Item, M, or L
    loop {
        match ctx.next() {
            Some(b"Item") => return true,
            Some(b"M") => return true,
            Some(b"L") => return true,
            Some(b"#top") | None => return false,
            Some(_) => continue,
        }
    }
}

fn find_action<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Match "Item" key at root
    if iter_match(|| [b"Item", b"#top"], structural, context.clone()) {
        return Some(on_item_begin);
    }

    // Match field keys - can be inside Item or inside M type objects
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
                    // Field inside M type object
                    // Need to distinguish from type keys inside a field named "M"
                    // Count consecutive M's: [field, M, M, M, ...]
                    // Even count of M's = innermost is a field name
                    // Odd count of M's = innermost is a type marker
                    else if parent == b"M" {
                        let mut m_count = 1;
                        let mut ctx_check = ctx.clone();
                        while let Some(next) = ctx_check.next() {
                            if next == b"M" {
                                m_count += 1;
                            } else {
                                break;
                            }
                        }
                        // Odd number of M's means this is a real M type object
                        if m_count % 2 == 1 {
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
            // Type descriptors under M or Item fields
            if first != b"#top" && first != b"#array" {
                if let Some(parent) = ctx.next() {
                    if parent == b"Item" || parent == b"M" {
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
                        _ => return None,
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
            // An M type object has context: [M, fieldName, Item/M/L, ...]
            // When we have [M, M, ...], need to count M's to determine
            // Odd number of consecutive M's = M type object
            // Even number = field named "M"
            if first == b"M" {
                if let Some(second) = ctx.next() {
                    if second == b"M" {
                        // Count consecutive M's after the first one
                        let mut m_count = 2; // first and second
                        let mut ctx_check = ctx.clone();
                        while let Some(next) = ctx_check.next() {
                            if next == b"M" {
                                m_count += 1;
                            } else {
                                break;
                            }
                        }
                        // Odd count = M type object ending
                        if m_count % 2 == 1 {
                            return Some(on_map_end);
                        }
                        // Even count = field named "M", fall through
                    } else if second == b"#array" {
                        // M object inside an L array
                        if let Some(parent) = ctx.next() {
                            if parent == b"L" {
                                return Some(on_map_end);
                            }
                        }
                    } else if second != b"#top" {
                        // This is a real M type object ending
                        return Some(on_map_end);
                    }
                    // Otherwise fall through to type_descriptor_end handling
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
///
/// # Arguments
/// * `reader` - Input stream implementing `embedded_io::Read`
/// * `writer` - Output stream implementing `embedded_io::Write`
/// * `rjiter_buffer` - Buffer for rjiter to use (recommended: 4096 bytes)
/// * `context_buffer` - Buffer for scan_json context tracking (recommended: 2048 bytes)
/// * `pretty` - Whether to pretty-print the output
///
/// # Returns
/// `Ok(())` on success, or an error message on failure
pub fn convert_ddb_to_normal<R: IoRead, W: IoWrite>(
    reader: &mut R,
    writer: &mut W,
    rjiter_buffer: &mut [u8],
    context_buffer: &mut [u8],
    pretty: bool,
) -> Result<(), &'static str> {
    let mut rjiter = RJiter::new(reader, rjiter_buffer);

    let converter = DdbConverter::new(writer, pretty);
    let baton = RefCell::new(converter);

    let mut context = U8Pool::new(context_buffer, 32)
        .map_err(|_| "Failed to create context pool")?;

    scan(
        find_action,
        find_end_action,
        &mut rjiter,
        &baton,
        &mut context,
        &Options::new(),
    )
    .map_err(|_| "Scan error")
}
