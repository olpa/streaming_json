use crate::ConversionError;
use core::cell::RefCell;
use embedded_io::{Read as IoRead, Write as IoWrite};
use rjiter::jiter::Peek;
use rjiter::RJiter;
use scan_json::matcher::StructuralPseudoname;
use scan_json::stack::ContextIter;
use scan_json::{scan, Action, EndAction, Options, StreamOp};
use u8pool::U8Pool;

pub struct NormalToDdbConverter<'a, 'workbuf, W: IoWrite> {
    writer: &'a mut W,
    pending_comma: bool,
    with_item_wrapper: bool,
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

fn on_root_object_begin<R: embedded_io::Read, W: IoWrite>(
    _rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
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
    StreamOp::None
}

fn on_field_key<R: embedded_io::Read, W: IoWrite>(
    _rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let field_name = {
        let conv = baton.borrow();
        conv.current_field
            .expect("current_field should be set")
            .to_vec()
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

fn on_string_value_toddb<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
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

fn on_bool_value_toddb<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let Ok(peek) = rjiter.peek() else {
        return StreamOp::Error("Failed to peek boolean value");
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

fn on_null_value_toddb<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let Ok(peek) = rjiter.peek() else {
        return StreamOp::Error("Failed to peek null value");
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

fn on_atom_value_toddb<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    // For atoms, we need to write {type:value} wrapper
    // The opening { is written here, closing } is written by type handlers
    {
        let mut conv = baton.borrow_mut();
        conv.write_comma();
        // Note: field handlers already wrote the opening {, so we don't write it for field values
        // But for array elements, we need it
    }

    // Peek to determine the actual type
    let Ok(peek) = rjiter.peek() else {
        return StreamOp::Error("Failed to peek atom value");
    };

    match peek {
        Peek::String => on_string_value_toddb(rjiter, baton),
        Peek::True | Peek::False => on_bool_value_toddb(rjiter, baton),
        Peek::Null => on_null_value_toddb(rjiter, baton),
        // Numbers: Int, Float, or any numeric peek type
        _ => {
            // Use next_number_bytes to preserve the exact string representation
            // This ensures "4.0" stays as "4.0" and doesn't become "4"
            let Ok(number_bytes) = rjiter.next_number_bytes() else {
                return StreamOp::Error("Failed to parse number");
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

fn on_array_begin_toddb<R: embedded_io::Read, W: IoWrite>(
    _rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.indent();
    conv.write(b"\"L\":[");
    conv.newline();
    conv.pending_comma = false;
    StreamOp::None
}

fn on_array_element_atom<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
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

fn on_array_element_array<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    // Array inside array - write element wrapper and L type
    {
        let mut conv = baton.borrow_mut();
        conv.write_comma();
        conv.write(b"{");
        conv.newline();
        conv.depth += 1;
        conv.pending_comma = false;
    }

    on_array_begin_toddb(rjiter, baton)
}

fn on_array_element_object<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    // Object inside array - write element wrapper and M type
    {
        let mut conv = baton.borrow_mut();
        conv.write_comma();
        conv.write(b"{");
        conv.newline();
        conv.depth += 1;
        conv.pending_comma = false;
    }

    on_nested_object_begin_toddb(rjiter, baton)
}

fn on_nested_object_begin_toddb<R: embedded_io::Read, W: IoWrite>(
    _rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
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

fn on_nested_object_end<W: IoWrite>(
    baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
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

fn on_array_element_end_toddb<W: IoWrite>(
    _baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
    // Close the element wrapper with } (for atoms only) - note that the value handler already closed and decreased depth
    // Value handler already wrote the closing } and decreased depth
    // Nothing more to do here
    Ok(())
}

fn on_object_in_array_end<W: IoWrite>(
    baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
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

fn on_array_in_array_end<W: IoWrite>(
    baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
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

fn find_action<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
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

            // Field key (root-level or nested)
            return Some(on_field_key);
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
            }
            // Object as a field value - write M type wrapper
            return Some(on_nested_object_begin_toddb);
        }
    }

    None
}

fn find_end_action<'a, 'workbuf, W: IoWrite>(
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
            // Any object that's not at the root and not in an array is a nested object
            return Some(on_nested_object_end);
        }
    }

    // Match end of arrays
    if structural == StructuralPseudoname::Array {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            if first == b"#array" {
                // Array inside another array - close with ]} and element wrapper }
                return Some(on_array_in_array_end);
            }
            // Root-level or field value array - close with ]}
            return Some(on_array_end_toddb);
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

/// Convert normal JSON to `DynamoDB` JSON in a streaming manner.
/// Supports JSONL format (newline-delimited JSON) - processes multiple JSON objects.
///
/// # Arguments
/// * `reader` - Input stream implementing `embedded_io::Read`
/// * `writer` - Output stream implementing `embedded_io::Write`
/// * `rjiter_buffer` - Buffer for rjiter to use (recommended: 4096 bytes)
/// * `context_buffer` - Buffer for `scan_json` context tracking (recommended: 2048 bytes)
/// * `pretty` - Whether to pretty-print the output (currently unused, may be added later)
/// * `with_item_wrapper` - Whether to wrap the output in an "Item" key
///
/// # Errors
/// Returns `ConversionError` if:
/// - Input JSON is malformed or invalid
/// - Input contains data types that cannot be represented in `DynamoDB` format
/// - I/O errors occur during reading or writing
/// - Buffer sizes are insufficient for the input data
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

    let mut context = U8Pool::new(context_buffer, 32).map_err(|_| {
        ConversionError::ScanError(scan_json::Error::InternalError {
            position: 0,
            message: "Failed to create context pool",
        })
    })?;

    scan(
        find_action,
        find_end_action,
        &mut rjiter,
        &baton,
        &mut context,
        &Options::new(),
    )
    .map_err(ConversionError::ScanError)?;

    Ok(())
}
