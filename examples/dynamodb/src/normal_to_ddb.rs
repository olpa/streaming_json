use crate::ConversionError;
use core::cell::RefCell;
use embedded_io::{Error as IoError, Read as IoRead, Write as IoWrite};
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

    fn store_io_error(
        &mut self,
        kind: embedded_io::ErrorKind,
        position: usize,
        context: &'static str,
    ) {
        self.last_error = Some(ConversionError::IOError {
            kind,
            position,
            context,
        });
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), embedded_io::ErrorKind> {
        self.writer.write_all(bytes).map_err(|e| e.kind())?;
        if self.unbuffered {
            self.writer.flush().map_err(|e| e.kind())?;
        }
        Ok(())
    }

    /// Helper that writes and stores error if write fails
    fn try_write(&mut self, bytes: &[u8], position: usize, context: &'static str) -> bool {
        if let Err(kind) = self.write(bytes) {
            self.store_io_error(kind, position, context);
            false
        } else {
            true
        }
    }

    fn write_comma(&mut self, position: usize) -> bool {
        if self.pending_comma {
            if !self.try_write(b",", position, "writing comma") {
                return false;
            }
            if !self.newline(position) {
                return false;
            }
            self.pending_comma = false;
        }
        true
    }

    fn newline(&mut self, position: usize) -> bool {
        if self.pretty {
            self.try_write(b"\n", position, "writing newline")
        } else {
            true
        }
    }

    fn indent(&mut self, position: usize) -> bool {
        if self.pretty {
            for _ in 0..self.depth {
                if !self.try_write(b"  ", position, "writing indentation") {
                    return false;
                }
            }
        }
        true
    }
}

type NormalToDdbBaton<'a, 'workbuf, W> = &'a RefCell<NormalToDdbConverter<'a, 'workbuf, W>>;

fn on_root_object_begin<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let position = rjiter.current_index();
    if conv.with_item_wrapper {
        if !conv.try_write(b"{", position, "writing root object opening brace") {
            return StreamOp::Error("Failed to write opening brace");
        }
        if !conv.newline(position) {
            return StreamOp::Error("Failed to write newline");
        }
        conv.depth += 1;
        if !conv.indent(position) {
            return StreamOp::Error("Failed to write indentation");
        }
        if !conv.try_write(b"\"Item\":{", position, "writing Item wrapper") {
            return StreamOp::Error("Failed to write Item wrapper");
        }
        if !conv.newline(position) {
            return StreamOp::Error("Failed to write newline");
        }
        conv.depth += 1;
    } else {
        if !conv.try_write(b"{", position, "writing root object opening brace") {
            return StreamOp::Error("Failed to write opening brace");
        }
        if !conv.newline(position) {
            return StreamOp::Error("Failed to write newline");
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
        let position = rjiter.current_index();
        if !conv.try_write(b"{", position, "writing root literal opening brace") {
            return StreamOp::Error("Failed to write opening brace");
        }
        conv.depth += 1;
    }
    let result = on_atom_value_toddb(rjiter, baton);
    // Atom handlers close the } and set pending_comma, but for root we need final newline
    {
        let mut conv = baton.borrow_mut();
        let position = rjiter.current_index();
        if !conv.try_write(b"\n", position, "writing final newline") {
            return StreamOp::Error("Failed to write final newline");
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
    let position = rjiter.current_index();
    if !conv.try_write(b"{", position, "writing root array opening brace") {
        return StreamOp::Error("Failed to write opening brace");
    }
    if !conv.newline(position) {
        return StreamOp::Error("Failed to write newline");
    }
    conv.depth += 1;
    drop(conv);
    on_array_begin_toddb(rjiter, baton)
}

fn on_root_array_end<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Close root-level array: write ]} and newline
    on_array_end_toddb(baton)?;
    let mut conv = baton.borrow_mut();
    if !conv.try_write(b"\n", 0, "writing final newline") {
        return Err("Failed to write final newline");
    }
    Ok(())
}

fn on_field_key<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let Some(field_name) = conv.current_field else {
        return StreamOp::Error("Internal error: current_field not set (impossible)");
    };
    let position = rjiter.current_index();
    if !conv.write_comma(position) {
        return StreamOp::Error("Failed to write comma");
    }
    if !conv.indent(position) {
        return StreamOp::Error("Failed to write indentation");
    }
    if !conv.try_write(b"\"", position, "writing field name opening quote") {
        return StreamOp::Error("Failed to write quote");
    }
    if !conv.try_write(field_name, position, "writing field name") {
        return StreamOp::Error("Failed to write field name");
    }
    if !conv.try_write(b"\":{", position, "writing field name closing quote and colon") {
        return StreamOp::Error("Failed to write quote and colon");
    }
    if !conv.newline(position) {
        return StreamOp::Error("Failed to write newline");
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
    let position = rjiter.current_index();
    if !conv.indent(position) {
        return StreamOp::Error("Failed to write indentation");
    }
    if !conv.try_write(b"\"S\":\"", position, "writing S type opening") {
        return StreamOp::Error("Failed to write S type opening");
    }
    if let Err(_) = rjiter.write_long_bytes(conv.writer) {
        return StreamOp::Error("Failed to write string value");
    }
    if !conv.try_write(b"\"", position, "writing S type closing quote") {
        return StreamOp::Error("Failed to write closing quote");
    }
    if !conv.newline(position) {
        return StreamOp::Error("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(position) {
        return StreamOp::Error("Failed to write indentation");
    }
    if !conv.try_write(b"}", position, "writing closing brace") {
        return StreamOp::Error("Failed to write closing brace");
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
    let position = rjiter.current_index();
    if !conv.indent(position) {
        return StreamOp::Error("Failed to write indentation");
    }
    match peek {
        Peek::True => {
            let _ = rjiter.known_bool(peek);
            if !conv.try_write(b"\"BOOL\":true", position, "writing BOOL true") {
                return StreamOp::Error("Failed to write BOOL true");
            }
        }
        Peek::False => {
            let _ = rjiter.known_bool(peek);
            if !conv.try_write(b"\"BOOL\":false", position, "writing BOOL false") {
                return StreamOp::Error("Failed to write BOOL false");
            }
        }
        _ => return StreamOp::Error("Expected boolean value"),
    }
    if !conv.newline(position) {
        return StreamOp::Error("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(position) {
        return StreamOp::Error("Failed to write indentation");
    }
    if !conv.try_write(b"}", position, "writing closing brace") {
        return StreamOp::Error("Failed to write closing brace");
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
    let position = rjiter.current_index();
    if !conv.indent(position) {
        return StreamOp::Error("Failed to write indentation");
    }
    if !conv.try_write(b"\"NULL\":true", position, "writing NULL type") {
        return StreamOp::Error("Failed to write NULL type");
    }
    if !conv.newline(position) {
        return StreamOp::Error("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(position) {
        return StreamOp::Error("Failed to write indentation");
    }
    if !conv.try_write(b"}", position, "writing closing brace") {
        return StreamOp::Error("Failed to write closing brace");
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
        let position = rjiter.current_index();
        if !conv.write_comma(position) {
            return StreamOp::Error("Failed to write comma");
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
            let position = rjiter.current_index();
            let Ok(number_bytes) = rjiter.next_number_bytes() else {
                return StreamOp::Error("Failed to parse number");
            };

            let mut conv = baton.borrow_mut();
            if !conv.indent(position) {
                return StreamOp::Error("Failed to write indentation");
            }
            if !conv.try_write(b"\"N\":\"", position, "writing N type opening") {
                return StreamOp::Error("Failed to write N type opening");
            }
            if !conv.try_write(number_bytes, position, "writing number value") {
                return StreamOp::Error("Failed to write number value");
            }
            if !conv.try_write(b"\"", position, "writing N type closing quote") {
                return StreamOp::Error("Failed to write closing quote");
            }
            if !conv.newline(position) {
                return StreamOp::Error("Failed to write newline");
            }
            conv.depth -= 1;
            if !conv.indent(position) {
                return StreamOp::Error("Failed to write indentation");
            }
            if !conv.try_write(b"}", position, "writing closing brace") {
                return StreamOp::Error("Failed to write closing brace");
            }
            conv.pending_comma = true;
            StreamOp::ValueIsConsumed
        }
    }
}

fn on_array_begin_toddb<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let position = rjiter.current_index();
    if !conv.indent(position) {
        return StreamOp::Error("Failed to write indentation");
    }
    if !conv.try_write(b"\"L\":[", position, "writing L type opening") {
        return StreamOp::Error("Failed to write L type opening");
    }
    if !conv.newline(position) {
        return StreamOp::Error("Failed to write newline");
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
        let position = rjiter.current_index();
        if !conv.write_comma(position) {
            return StreamOp::Error("Failed to write comma");
        }
        if !conv.try_write(b"{", position, "writing array element opening brace") {
            return StreamOp::Error("Failed to write opening brace");
        }
        if !conv.newline(position) {
            return StreamOp::Error("Failed to write newline");
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
        let position = rjiter.current_index();
        if !conv.write_comma(position) {
            return StreamOp::Error("Failed to write comma");
        }
        if !conv.try_write(b"{", position, "writing array element opening brace") {
            return StreamOp::Error("Failed to write opening brace");
        }
        if !conv.newline(position) {
            return StreamOp::Error("Failed to write newline");
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
        let position = rjiter.current_index();
        if !conv.write_comma(position) {
            return StreamOp::Error("Failed to write comma");
        }
        if !conv.try_write(b"{", position, "writing array element opening brace") {
            return StreamOp::Error("Failed to write opening brace");
        }
        if !conv.newline(position) {
            return StreamOp::Error("Failed to write newline");
        }
        conv.depth += 1;
        conv.pending_comma = false;
    }

    on_nested_object_begin_toddb(rjiter, baton)
}

fn on_nested_object_begin_toddb<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: NormalToDdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let position = rjiter.current_index();
    if !conv.indent(position) {
        return StreamOp::Error("Failed to write indentation");
    }
    if !conv.try_write(b"\"M\":{", position, "writing M type opening") {
        return StreamOp::Error("Failed to write M type opening");
    }
    if !conv.newline(position) {
        return StreamOp::Error("Failed to write newline");
    }
    conv.depth += 1;
    conv.pending_comma = false;
    StreamOp::None
}

#[allow(clippy::unnecessary_wraps)]
fn on_root_object_end<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    if !conv.newline(0) {
        return Err("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(0) {
        return Err("Failed to write indentation");
    }
    if conv.with_item_wrapper {
        if !conv.try_write(b"}", 0, "writing root object closing brace") {
            return Err("Failed to write closing brace");
        }
        if !conv.newline(0) {
            return Err("Failed to write newline");
        }
        conv.depth -= 1;
        if !conv.indent(0) {
            return Err("Failed to write indentation");
        }
        if !conv.try_write(b"}", 0, "writing Item wrapper closing brace") {
            return Err("Failed to write closing brace");
        }
    } else {
        if !conv.try_write(b"}", 0, "writing root object closing brace") {
            return Err("Failed to write closing brace");
        }
    }
    if !conv.try_write(b"\n", 0, "writing final newline") {
        return Err("Failed to write final newline");
    }
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_nested_object_end<W: IoWrite>(
    baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    if !conv.newline(0) {
        return Err("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(0) {
        return Err("Failed to write indentation");
    }
    if !conv.try_write(b"}", 0, "writing M closing brace") {
        return Err("Failed to write closing brace");
    }
    if !conv.newline(0) {
        return Err("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(0) {
        return Err("Failed to write indentation");
    }
    if !conv.try_write(b"}", 0, "writing element wrapper closing brace") {
        return Err("Failed to write closing brace");
    }
    conv.pending_comma = true;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_array_end_toddb<W: IoWrite>(baton: NormalToDdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    if !conv.newline(0) {
        return Err("Failed to write newline");
    }
    if !conv.indent(0) {
        return Err("Failed to write indentation");
    }
    if !conv.try_write(b"]", 0, "writing L closing bracket") {
        return Err("Failed to write closing bracket");
    }
    if !conv.newline(0) {
        return Err("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(0) {
        return Err("Failed to write indentation");
    }
    if !conv.try_write(b"}", 0, "writing element wrapper closing brace") {
        return Err("Failed to write closing brace");
    }
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
    if !conv.newline(0) {
        return Err("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(0) {
        return Err("Failed to write indentation");
    }
    if !conv.try_write(b"}", 0, "writing M closing brace") {
        return Err("Failed to write closing brace");
    }
    if !conv.newline(0) {
        return Err("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(0) {
        return Err("Failed to write indentation");
    }
    if !conv.try_write(b"}", 0, "writing element wrapper closing brace") {
        return Err("Failed to write closing brace");
    }
    conv.pending_comma = true;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_array_in_array_end<W: IoWrite>(
    baton: NormalToDdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
    // Close the L array with ] and the element wrapper with }
    let mut conv = baton.borrow_mut();
    if !conv.newline(0) {
        return Err("Failed to write newline");
    }
    if !conv.indent(0) {
        return Err("Failed to write indentation");
    }
    if !conv.try_write(b"]", 0, "writing L closing bracket") {
        return Err("Failed to write closing bracket");
    }
    if !conv.newline(0) {
        return Err("Failed to write newline");
    }
    conv.depth -= 1;
    if !conv.indent(0) {
        return Err("Failed to write indentation");
    }
    if !conv.try_write(b"}", 0, "writing element wrapper closing brace") {
        return Err("Failed to write closing brace");
    }
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
        let stored_error = baton.borrow().last_error.clone();
        if let Some(err) = stored_error {
            return Err(err);
        }
        // Otherwise return the scan error
        return Err(ConversionError::ScanError(e));
    }

    Ok(())
}
