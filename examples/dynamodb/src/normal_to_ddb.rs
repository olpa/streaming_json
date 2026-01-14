use crate::ConversionError;
use core::cell::RefCell;
use embedded_io::{Error as IoError, Read as IoRead, Write as IoWrite};
use rjiter::jiter::Peek;
use rjiter::RJiter;
use scan_json::matcher::StructuralPseudoname;
use scan_json::stack::ContextIter;
use scan_json::{scan, Action, EndAction, Options, StreamOp};
use u8pool::U8Pool;

/// Sentinel value indicating position will be updated later from scan_json
const POSITION_UPDATED_LATER: usize = usize::MAX;

pub struct NormalToDdbConverter<'a, 'workbuf, W: IoWrite> {
    writer: &'a mut W,
    pending_comma: bool,
    with_item_wrapper: bool,
    unbuffered: bool,
    current_field: Option<&'workbuf [u8]>,
    pretty: bool,
    depth: usize,
    last_error: Option<ConversionError>,
}

impl<'a, W: IoWrite> NormalToDdbConverter<'a, '_, W> {
    fn new(writer: &'a mut W, with_item_wrapper: bool, pretty: bool, unbuffered: bool) -> Self {
        Self {
            writer,
            pending_comma: false,
            with_item_wrapper,
            unbuffered,
            current_field: None,
            pretty,
            depth: 0,
            last_error: None,
        }
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), embedded_io::ErrorKind> {
        self.writer.write_all(bytes).map_err(|e| e.kind())?;
        if self.unbuffered {
            self.writer.flush().map_err(|e| e.kind())?;
        }
        Ok(())
    }

    /// Helper that writes and stores error without position
    /// Position will be added later from scan_json's ActionError which calls rjiter.current_index()
    fn try_write_any(&mut self, bytes: &[u8], context: &'static str) -> Result<(), &'static str> {
        self.write(bytes).map_err(|kind| {
            // Store error with sentinel position - scan_json will provide accurate position
            self.last_error = Some(ConversionError::IOError {
                kind,
                position: POSITION_UPDATED_LATER,
                context,
            });
            "Write failed"
        })
    }

    fn write_comma(&mut self) -> Result<(), &'static str> {
        if self.pending_comma {
            self.try_write_any(b",", "writing comma")?;
            self.newline()?;
            self.pending_comma = false;
        }
        Ok(())
    }

    fn newline(&mut self) -> Result<(), &'static str> {
        if self.pretty {
            self.try_write_any(b"\n", "writing newline")
        } else {
            Ok(())
        }
    }

    fn indent(&mut self) -> Result<(), &'static str> {
        if self.pretty {
            for _ in 0..self.depth {
                self.try_write_any(b"  ", "writing indentation")?;
            }
        }
        Ok(())
    }
}

type NormalToDdbBaton<'a, 'workbuf, W> = &'a RefCell<NormalToDdbConverter<'a, 'workbuf, W>>;

fn on_root_object_begin<R: embedded_io::Read, W: IoWrite>(
    _rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    if conv.with_item_wrapper {
        if let Err(e) = conv.try_write_any(b"{", "writing root object opening brace") {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.newline() {
            return StreamOp::Error(e);
        }
        conv.depth += 1;
        if let Err(e) = conv.indent() {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.try_write_any(b"\"Item\":{", "writing Item wrapper") {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.newline() {
            return StreamOp::Error(e);
        }
        conv.depth += 1;
    } else {
        if let Err(e) = conv.try_write_any(b"{", "writing root object opening brace") {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.newline() {
            return StreamOp::Error(e);
        }
        conv.depth += 1;
    }
    conv.pending_comma = false;
    StreamOp::None
}

fn on_root_literal<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    // Root-level primitive (string, number, boolean, null)
    // Write opening { and increment depth, then delegate to atom handler
    {
        let mut conv = baton.borrow_mut();
        if let Err(e) = conv.try_write_any(b"{", "writing root literal opening brace") {
            return StreamOp::Error(e);
        }
        conv.depth += 1;
    }
    let result = on_atom_value_toddb(rjiter, baton);
    // Atom handlers close the } and set pending_comma, but for root we need final newline
    {
        let mut conv = baton.borrow_mut();
        if let Err(e) = conv.try_write_any(b"\n", "writing final newline") {
            return StreamOp::Error(e);
        }
        conv.pending_comma = false;
    }
    result
}

fn on_root_array<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    // Root-level array: write opening { and increment depth, then delegate
    let mut conv = baton.borrow_mut();
    if let Err(e) = conv.try_write_any(b"{", "writing root array opening brace") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.newline() {
        return StreamOp::Error(e);
    }
    conv.depth += 1;
    drop(conv);
    on_array_begin_toddb(rjiter, baton)
}

fn on_root_array_end<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Close root-level array: write ]} and newline
    on_array_end_toddb(baton)?;
    let mut conv = baton.borrow_mut();
    conv.try_write_any(b"\n", "writing final newline")?;
    Ok(())
}

fn on_field_key<R: embedded_io::Read, W: IoWrite>(
    _rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let Some(field_name) = conv.current_field else {
        return StreamOp::Error("Internal error: current_field not set (impossible)");
    };
    if let Err(e) = conv.write_comma() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.indent() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"\"", "writing field name opening quote") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(field_name, "writing field name") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"\":{", "writing field name closing quote and colon") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.newline() {
        return StreamOp::Error(e);
    }
    conv.depth += 1;
    conv.pending_comma = false;
    StreamOp::None
}

fn on_string_value_toddb<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    if let Err(e) = conv.indent() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"\"S\":\"", "writing S type opening") {
        return StreamOp::Error(e);
    }
    if let Err(_) = rjiter.write_long_bytes(conv.writer) {
        return StreamOp::Error("Failed to write string value");
    }
    if conv.unbuffered {
        if let Err(e) = conv.writer.flush() {
            conv.last_error = Some(ConversionError::IOError {
                kind: e.kind(),
                position: POSITION_UPDATED_LATER,
                context: "flushing after write_long_bytes",
            });
            return StreamOp::Error("Failed to flush writer");
        }
    }
    if let Err(e) = conv.try_write_any(b"\"", "writing S type closing quote") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.newline() {
        return StreamOp::Error(e);
    }
    conv.depth -= 1;
    if let Err(e) = conv.indent() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"}", "writing closing brace") {
        return StreamOp::Error(e);
    }
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
    if let Err(e) = conv.indent() {
        return StreamOp::Error(e);
    }
    match peek {
        Peek::True => {
            let _ = rjiter.known_bool(peek);
            if let Err(e) = conv.try_write_any(b"\"BOOL\":true", "writing BOOL true") {
                return StreamOp::Error(e);
            }
        }
        Peek::False => {
            let _ = rjiter.known_bool(peek);
            if let Err(e) = conv.try_write_any(b"\"BOOL\":false", "writing BOOL false") {
                return StreamOp::Error(e);
            }
        }
        _ => return StreamOp::Error("Expected boolean value"),
    }
    if let Err(e) = conv.newline() {
        return StreamOp::Error(e);
    }
    conv.depth -= 1;
    if let Err(e) = conv.indent() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"}", "writing closing brace") {
        return StreamOp::Error(e);
    }
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
    if let Err(e) = conv.indent() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"\"NULL\":true", "writing NULL type") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.newline() {
        return StreamOp::Error(e);
    }
    conv.depth -= 1;
    if let Err(e) = conv.indent() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"}", "writing closing brace") {
        return StreamOp::Error(e);
    }
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
        if let Err(e) = conv.write_comma() {
            return StreamOp::Error(e);
        }
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
            // Copy the bytes so we can use rjiter later
            let number_bytes = match rjiter.next_number_bytes() {
                Ok(bytes) => {
                    let mut buf = [0u8; 32];
                    let len = bytes.len().min(32);
                    buf[..len].copy_from_slice(&bytes[..len]);
                    (buf, len)
                }
                Err(_) => return StreamOp::Error("Failed to parse number"),
            };

            let mut conv = baton.borrow_mut();
            if let Err(e) = conv.indent() {
                return StreamOp::Error(e);
            }
            if let Err(e) = conv.try_write_any(b"\"N\":\"", "writing N type opening") {
                return StreamOp::Error(e);
            }
            if let Err(e) = conv.try_write_any(&number_bytes.0[..number_bytes.1], "writing number value") {
                return StreamOp::Error(e);
            }
            if let Err(e) = conv.try_write_any(b"\"", "writing N type closing quote") {
                return StreamOp::Error(e);
            }
            if let Err(e) = conv.newline() {
                return StreamOp::Error(e);
            }
            conv.depth -= 1;
            if let Err(e) = conv.indent() {
                return StreamOp::Error(e);
            }
            if let Err(e) = conv.try_write_any(b"}", "writing closing brace") {
                return StreamOp::Error(e);
            }
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
    if let Err(e) = conv.indent() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"\"L\":[", "writing L type opening") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.newline() {
        return StreamOp::Error(e);
    }
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
        if let Err(e) = conv.write_comma() {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.try_write_any(b"{", "writing array element opening brace") {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.newline() {
            return StreamOp::Error(e);
        }
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
        if let Err(e) = conv.write_comma() {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.try_write_any(b"{", "writing array element opening brace") {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.newline() {
            return StreamOp::Error(e);
        }
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
        if let Err(e) = conv.write_comma() {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.try_write_any(b"{", "writing array element opening brace") {
            return StreamOp::Error(e);
        }
        if let Err(e) = conv.newline() {
            return StreamOp::Error(e);
        }
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
    if let Err(e) = conv.indent() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"\"M\":{", "writing M type opening") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.newline() {
        return StreamOp::Error(e);
    }
    conv.depth += 1;
    conv.pending_comma = false;
    StreamOp::None
}

#[allow(clippy::unnecessary_wraps)]
fn on_root_object_end<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline()?;
    conv.depth -= 1;
    conv.indent()?;
    if conv.with_item_wrapper {
        conv.try_write_any(b"}", "writing root object closing brace")?;
        conv.newline()?;
        conv.depth -= 1;
        conv.indent()?;
        conv.try_write_any(b"}", "writing Item wrapper closing brace")?;
    } else {
        conv.try_write_any(b"}", "writing root object closing brace")?;
    }
    conv.try_write_any(b"\n", "writing final newline")?;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_nested_object_end<W: IoWrite>(
    baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline()?;
    conv.depth -= 1;
    conv.indent()?;
    conv.try_write_any(b"}", "writing M closing brace")?;
    conv.newline()?;
    conv.depth -= 1;
    conv.indent()?;
    conv.try_write_any(b"}", "writing element wrapper closing brace")?;
    conv.pending_comma = true;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_array_end_toddb<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline()?;
    conv.indent()?;
    conv.try_write_any(b"]", "writing L closing bracket")?;
    conv.newline()?;
    conv.depth -= 1;
    conv.indent()?;
    conv.try_write_any(b"}", "writing element wrapper closing brace")?;
    conv.pending_comma = true;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_array_element_end_toddb<W: IoWrite>(
    _baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
    // Close the element wrapper with } (for atoms only) - note that the value handler already closed and decreased depth
    // Value handler already wrote the closing } and decreased depth
    // Nothing more to do here
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_object_in_array_end<W: IoWrite>(
    baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
    // Close the M object with } and then close the element wrapper with }
    let mut conv = baton.borrow_mut();
    conv.newline()?;
    conv.depth -= 1;
    conv.indent()?;
    conv.try_write_any(b"}", "writing M closing brace")?;
    conv.newline()?;
    conv.depth -= 1;
    conv.indent()?;
    conv.try_write_any(b"}", "writing element wrapper closing brace")?;
    conv.pending_comma = true;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_array_in_array_end<W: IoWrite>(
    baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
    // Close the L array with ] and the element wrapper with }
    let mut conv = baton.borrow_mut();
    conv.newline()?;
    conv.indent()?;
    conv.try_write_any(b"]", "writing L closing bracket")?;
    conv.newline()?;
    conv.depth -= 1;
    conv.indent()?;
    conv.try_write_any(b"}", "writing element wrapper closing brace")?;
    conv.pending_comma = true;
    Ok(())
}

fn find_action<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    structural: StructuralPseudoname,
    mut context: ContextIter,
    baton: NormalToDdbBaton<'a, 'workbuf, W>,
) -> Option<Action<NormalToDdbBaton<'a, 'workbuf, W>, R>> {
    // Get the parent context element once, used by all branches below
    let parent = context.next();

    // Match root-level values
    if parent == Some(b"#top") {
        return match structural {
            StructuralPseudoname::Object => Some(on_root_object_begin),
            StructuralPseudoname::Atom => Some(on_root_literal),
            StructuralPseudoname::Array => Some(on_root_array),
            StructuralPseudoname::None => None,
        };
    }

    // Match field keys
    if structural == StructuralPseudoname::None {
        if let Some(field) = parent {
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

    // Check if we're in an array context
    let in_array = parent == Some(b"#array");

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
        if parent == Some(b"#top") {
            // Root object (already handled above)
            return None;
        } else if parent == Some(b"#array") {
            // Object in array - write element wrapper
            return Some(on_array_element_object);
        } else if parent.is_some() {
            // Object as a field value - write M type wrapper
            return Some(on_nested_object_begin_toddb);
        }
    }

    None
}

fn find_end_action<'a, 'workbuf, W: IoWrite>(
    structural: StructuralPseudoname,
    mut context: ContextIter,
    _baton: NormalToDdbBaton<'a, 'workbuf, W>,
) -> Option<EndAction<NormalToDdbBaton<'a, 'workbuf, W>>> {
    // Get the parent context element once
    let parent = context.next();

    // Match end of root-level values
    if parent == Some(b"#top") {
        return match structural {
            StructuralPseudoname::Object => Some(on_root_object_end),
            StructuralPseudoname::Array => Some(on_root_array_end),
            // Atoms don't have end actions (handled entirely in action)
            _ => None,
        };
    }

    // Match end of objects
    if structural == StructuralPseudoname::Object {
        // Objects in arrays - need to close both object and element wrapper
        if parent == Some(b"#array") {
            return Some(on_object_in_array_end);
        }
        // Nested objects (not in arrays)
        if parent.is_some() {
            // Any object that's not at the root and not in an array is a nested object
            return Some(on_nested_object_end);
        }
    }

    // Match end of arrays
    if structural == StructuralPseudoname::Array {
        if parent == Some(b"#array") {
            // Array inside another array - close with ]} and element wrapper }
            return Some(on_array_in_array_end);
        }
        if parent.is_some() {
            // Root-level or field value array - close with ]}
            return Some(on_array_end_toddb);
        }
    }

    // Match end of array elements (primitives)
    if structural == StructuralPseudoname::Atom && parent == Some(b"#array") {
        if let Some(grandparent) = context.next() {
            if grandparent == b"L" {
                return Some(on_array_element_end_toddb);
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
/// * `unbuffered` - Whether to flush after every write
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
    unbuffered: bool,
    with_item_wrapper: bool,
) -> Result<(), ConversionError> {
    let mut rjiter = RJiter::new(reader, rjiter_buffer);

    let converter = NormalToDdbConverter::new(writer, with_item_wrapper, pretty, unbuffered);
    let baton = RefCell::new(converter);

    // DynamoDB supports up to 32 levels of nesting.
    // Context stores: "#top" + field names at each level + final field
    // For 32 levels: 1 (#top) + 32 (level_N) + 1 (final field) = 34 slots
    let mut context = U8Pool::new(context_buffer, 34).map_err(|_| {
        ConversionError::ScanError(scan_json::Error::InternalError {
            position: 0,
            message: "Failed to create context pool",
        })
    })?;

    if let Err(e) = scan(
        find_action,
        find_end_action,
        &mut rjiter,
        &baton,
        &mut context,
        &Options::new(),
    ) {
        // Check if there's a stored detailed error in the baton
        let stored_error = baton.borrow_mut().last_error.take();
        if let Some(mut err) = stored_error {
            // Extract position from scan_json's error and update our stored error
            // scan_json calls rjiter.current_index() which gives accurate position
            let position = match &e {
                scan_json::Error::ActionError { position, .. } => *position,
                scan_json::Error::MaxNestingExceeded { position, .. } => *position,
                scan_json::Error::InternalError { position, .. } => *position,
                scan_json::Error::UnhandledPeek { position, .. } => *position,
                scan_json::Error::UnbalancedJson(position) => *position,
                scan_json::Error::RJiterError(e) => e.index,
                scan_json::Error::IOError(_) => rjiter.current_index(), // Get position from rjiter
            };
            // Update position in stored error (replaces POSITION_UPDATED_LATER sentinel)
            match &mut err {
                ConversionError::IOError { position: p, .. } => *p = position,
                ConversionError::RJiterError { position: p, .. } => *p = position,
                ConversionError::ParseError { position: p, .. } => *p = position,
                ConversionError::ScanError(_) => {}
            }
            return Err(err);
        }
        // Otherwise return the scan error
        return Err(ConversionError::ScanError(e));
    }

    Ok(())
}
