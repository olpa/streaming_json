//! Copy JSON input to output, retaining the original structure and collapsing whitespace.
//! The implementation of `idtransform` is an example of advanced use of the `scan` function.

//
// The code uses the `scan`'s parameter `baton` of type `IdTransform` to:
// - maintain state to properly write JSON, adding or not adding a comma, `IdtSequencePos`
// - pass information from matchers to handlers, `IdtMatcherToHandler`
//
//   Actually, there is no such thing as `IdtMatcherToHandler`, because doing "clean code"
//   and "good design" complicated the code too much. In the final implementation,
//   `IdTransform` took over the responsibility of the retired `IdtMatcherForKey`.
//
//   Why pass the information at all? This is another trade-off.
//   The code uses a match-any-key matcher. The matched key should be printed to the output.
//   In the current implementation of `scan`:
// - The matcher is not allowed to print anything.
// - The handler doesn't know the key.
//
//   How to print? The solution space is:
// - Allow matchers to print.
//   In this case, the return type of `scan` should be `Result`, not just `boolean`.
// - Pass the key to the handler.
//   In this case, the argument list of a handler should be extended, and `scan` should
//   pass the context which was passed to the matcher once again, now to the handler.
// - Pass the key from the matcher to the handler.
//   In this case: 1) the matcher produces a side effect, 2) the printing is postponed
//   to some unknown point in the future.
//
use crate::matcher::StructuralPseudoname;
use crate::stack::ContextIter;
use crate::StreamOp;
use crate::{
    rjiter::jiter::Peek, scan, Action, EndAction, Error as ScanError, Options, RJiter,
    Result as ScanResult,
};
use core::cell::RefCell;
use core::mem::transmute;
use embedded_io::{Error as EmbeddedError, Read, Write};

use u8pool::U8Pool;

/// Macro to write to the writer and store IO error on failure
macro_rules! write_and_store_error {
    ($idt:expr, $buf:expr, $msg:expr) => {
        $idt.writer.write_all($buf).map_err(|e| {
            $idt.io_error = Some(e.kind());
            $msg
        })
    };
}

/// Type alias for the baton type used in idtransform
type IdtBaton<'a, 'workbuf, W> = &'a RefCell<IdTransform<'a, 'workbuf, W>>;

/// Copy a JSON atom (string, number, boolean, or null) from the input to the output.
/// Advances the input iterator to the next token.
///
/// # Errors
///
/// This function will return an error if:
/// * The input JSON is malformed
/// * An IO error occurs while writing to the output
/// * An unexpected token type is encountered
pub fn copy_atom<R: Read, W: Write>(
    peeked: Peek,
    rjiter: &mut RJiter<R>,
    writer: &mut W,
) -> ScanResult<()> {
    if peeked == Peek::String {
        writer
            .write_all(b"\"")
            .map_err(|e| ScanError::IOError(e.kind()))?;
        rjiter.write_long_bytes(writer)?;
        writer
            .write_all(b"\"")
            .map_err(|e| ScanError::IOError(e.kind()))?;
        return Ok(());
    }
    if peeked == Peek::Null {
        rjiter.known_null()?;
        writer
            .write_all(b"null")
            .map_err(|e| ScanError::IOError(e.kind()))?;
        return Ok(());
    }
    if peeked == Peek::True {
        rjiter.known_bool(peeked)?;
        writer
            .write_all(b"true")
            .map_err(|e| ScanError::IOError(e.kind()))?;
        return Ok(());
    }
    if peeked == Peek::False {
        rjiter.known_bool(peeked)?;
        writer
            .write_all(b"false")
            .map_err(|e| ScanError::IOError(e.kind()))?;
        return Ok(());
    }
    let maybe_number = rjiter.next_number_bytes();
    if let Ok(number) = maybe_number {
        writer
            .write_all(number)
            .map_err(|e| ScanError::IOError(e.kind()))?;
        return Ok(());
    }
    Err(ScanError::UnhandledPeek {
        peek: peeked,
        position: rjiter.current_index(),
    })
}

// ---------------- State

#[derive(Debug)]
enum IdtSequencePos<'a> {
    AtBeginning,
    InMiddle,
    AtBeginningKey(&'a [u8]),
    InMiddleKey(&'a [u8]),
    AfterKey,
}

// Main transformer structure that maintains the state of the transformation process.
struct IdTransform<'a, 'workbuf, W: Write> {
    writer: &'a mut W,
    // `seqpos`+`is_top_level` could be the own type `IdtFromMatcherToHandler`
    seqpos: IdtSequencePos<'workbuf>,
    is_top_level: bool,
    io_error: Option<embedded_io::ErrorKind>,
    rjiter_error: Option<rjiter::Error>,
    scan_error: Option<ScanError>,
}

#[allow(clippy::elidable_lifetime_names)]
impl<'a, 'workbuf, W: Write> IdTransform<'a, 'workbuf, W> {
    fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            seqpos: IdtSequencePos::AtBeginning,
            is_top_level: true,
            io_error: None,
            rjiter_error: None,
            scan_error: None,
        }
    }

    fn get_writer_mut(&mut self) -> &mut W {
        self.writer
    }

    fn is_top_level(&self) -> bool {
        self.is_top_level
    }

    fn write_seqpos(&mut self) -> Result<(), &'static str> {
        match &self.seqpos {
            IdtSequencePos::AtBeginning => {
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::InMiddle => {
                let seqpos = if self.is_top_level() { b" " } else { b"," };
                write_and_store_error!(self, seqpos, "IO error writing sequence position")?;
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::AtBeginningKey(key) => {
                write_and_store_error!(self, b"\"", "IO error writing key quote")?;
                write_and_store_error!(self, key, "IO error writing key")?;
                write_and_store_error!(self, b"\":", "IO error writing key suffix")?;
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::InMiddleKey(key) => {
                write_and_store_error!(self, b",\"", "IO error writing key prefix")?;
                write_and_store_error!(self, key, "IO error writing key")?;
                write_and_store_error!(self, b"\":", "IO error writing key suffix")?;
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::AfterKey => Ok(()),
        }
    }
}

// ---------------- Matchers

fn find_action<'a, 'workbuf, R: Read, W: Write>(
    structural_pseudoname: StructuralPseudoname,
    mut context: ContextIter,
    baton: IdtBaton<'a, 'workbuf, W>,
) -> Option<Action<IdtBaton<'a, 'workbuf, W>, R>> {
    let context_count = context.len();
    match structural_pseudoname {
        StructuralPseudoname::Atom => {
            // Handle context for is_top_level
            let mut idt = baton.borrow_mut();
            idt.is_top_level = context_count < 2;
            Some(on_atom)
        }
        StructuralPseudoname::Object => {
            let mut idt = baton.borrow_mut();
            idt.is_top_level = context_count < 2;
            Some(on_object)
        }
        StructuralPseudoname::Array => {
            let mut idt = baton.borrow_mut();
            idt.is_top_level = context_count < 2;
            Some(on_array)
        }
        StructuralPseudoname::None => {
            // Handle key matching - for object keys, the key name is in the context path
            if let Some(key_bytes) = context.next() {
                if key_bytes != b"#top" && key_bytes != b"#array" {
                    let mut idt = baton.borrow_mut();
                    // Use unsafe to store the slice reference - we know it's safe because
                    // the working buffer outlives the IdTransform
                    #[allow(unsafe_code)]
                    let key_slice: &'workbuf [u8] =
                        unsafe { transmute::<&[u8], &'workbuf [u8]>(key_bytes) };
                    idt.seqpos = match &idt.seqpos {
                        IdtSequencePos::AtBeginning => IdtSequencePos::AtBeginningKey(key_slice),
                        _ => IdtSequencePos::InMiddleKey(key_slice),
                    };
                    Some(on_key)
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

fn find_end_action<'a, 'workbuf, W: Write>(
    structural_pseudoname: StructuralPseudoname,
    _context: ContextIter,
    _baton: IdtBaton<'a, 'workbuf, W>,
) -> Option<EndAction<IdtBaton<'a, 'workbuf, W>>> {
    match structural_pseudoname {
        StructuralPseudoname::Object => Some(on_object_end),
        StructuralPseudoname::Array => Some(on_array_end),
        StructuralPseudoname::Atom | StructuralPseudoname::None => None,
    }
}

// ---------------- Handlers

fn on_key<R: Read, W: Write>(
    _rjiter: &mut RJiter<R>,
    idt_cell: &RefCell<IdTransform<'_, '_, W>>,
) -> StreamOp {
    let mut idt = idt_cell.borrow_mut();

    if let Err(message) = idt.write_seqpos() {
        return StreamOp::Error(message);
    }
    idt.seqpos = IdtSequencePos::AfterKey;

    StreamOp::None
}

fn on_atom<R: Read, W: Write>(
    rjiter: &mut RJiter<R>,
    idt_cell: &RefCell<IdTransform<'_, '_, W>>,
) -> StreamOp {
    let mut idt = idt_cell.borrow_mut();

    if let Err(message) = idt.write_seqpos() {
        return StreamOp::Error(message);
    }

    match rjiter.peek() {
        Ok(peeked) => match copy_atom(peeked, rjiter, idt.get_writer_mut()) {
            Ok(()) => StreamOp::ValueIsConsumed,
            Err(e) => {
                // Store the error for later retrieval
                match e {
                    ScanError::IOError(kind) => {
                        idt.io_error = Some(kind);
                    }
                    ScanError::RJiterError(rjiter_error) => {
                        // Store IO error kind if it's an IoError
                        if let rjiter::error::ErrorType::IoError { kind } = rjiter_error.error_type
                        {
                            idt.io_error = Some(kind);
                        }
                        idt.rjiter_error = Some(rjiter_error);
                    }
                    other_error => {
                        idt.scan_error = Some(other_error);
                    }
                }
                StreamOp::Error("Error copying atom (stored in idt)")
            }
        },
        Err(e) => {
            // Store IO error kind if peek failed with an IO error
            if let rjiter::error::ErrorType::IoError { kind } = e.error_type {
                idt.io_error = Some(kind);
            }
            idt.rjiter_error = Some(e);
            StreamOp::Error("RJiter error (stored in idt)")
        }
    }
}

// "Struct" means "array" or "object"
fn on_struct<W: Write>(bytes: &[u8], idt_cell: &RefCell<IdTransform<'_, '_, W>>) -> StreamOp {
    let mut idt = idt_cell.borrow_mut();

    if let Err(message) = idt.write_seqpos() {
        return StreamOp::Error(message);
    }

    if let Err(message) = write_and_store_error!(idt, bytes, "IO error writing struct") {
        return StreamOp::Error(message);
    }
    idt.seqpos = IdtSequencePos::AtBeginning;
    StreamOp::None
}

fn on_struct_end<W: Write>(
    bytes: &[u8],
    idt_cell: &RefCell<IdTransform<'_, '_, W>>,
) -> Result<(), &'static str> {
    let mut idt = idt_cell.borrow_mut();
    idt.seqpos = IdtSequencePos::InMiddle;
    write_and_store_error!(idt, bytes, "IO error writing struct end")?;
    Ok(())
}

fn on_array<R: Read, W: Write>(
    _rjiter: &mut RJiter<R>,
    idt_cell: &RefCell<IdTransform<'_, '_, W>>,
) -> StreamOp {
    on_struct(b"[", idt_cell)
}

fn on_array_end<W: Write>(idt_cell: &RefCell<IdTransform<'_, '_, W>>) -> Result<(), &'static str> {
    on_struct_end(b"]", idt_cell)
}

fn on_object<R: Read, W: Write>(
    _rjiter: &mut RJiter<R>,
    idt_cell: &RefCell<IdTransform<'_, '_, W>>,
) -> StreamOp {
    on_struct(b"{", idt_cell)
}

fn on_object_end<W: Write>(idt_cell: &RefCell<IdTransform<'_, '_, W>>) -> Result<(), &'static str> {
    on_struct_end(b"}", idt_cell)
}

// ---------------- Entry point

/// Copy JSON input to output, retaining the original structure and collapsing whitespace.
///
/// The implementation of `idtransform` is an example of how to use the `scan` function.
/// Consult the source code for more details.
///
/// # Arguments
///
/// * `rjiter` - Mutable reference to the JSON iterator
/// * `writer` - Output writer for the transformed JSON
/// * `working_buffer` - Working buffer for context stack (see [`crate::scan()`] for details)
///
/// # Errors
///
/// If `scan` fails (malformed json, nesting too deep, etc), return `scan`'s error.
/// Also, if an IO error occurs while writing to the output, return it.
///
pub fn idtransform<R: Read, W: Write>(
    rjiter: &mut RJiter<R>,
    writer: &mut W,
    working_buffer: &mut U8Pool,
) -> ScanResult<()> {
    let idt = IdTransform::new(writer);
    let idt_cell = RefCell::new(idt);

    let scan_result = scan(
        find_action,
        find_end_action,
        rjiter,
        &idt_cell,
        working_buffer,
        &Options {
            sse_tokens: &[],
            stop_early: true,
        },
    );

    // Check if scan failed and if we have stored errors
    if let Err(scan_error) = scan_result {
        let idt = idt_cell.borrow();

        // Priority 1: If we have stored an IO error kind, return it
        if let Some(io_error_kind) = idt.io_error {
            return Err(ScanError::IOError(io_error_kind));
        }

        // Priority 2: If we have stored a RJiter error, return it
        if let Some(ref rjiter_error) = idt.rjiter_error {
            return Err(ScanError::RJiterError(rjiter_error.clone()));
        }

        // Priority 3: If we have stored another scan error, return it
        if let Some(ref stored_scan_error) = idt.scan_error {
            return Err(stored_scan_error.clone());
        }

        // Priority 4: Return the original scan error (should not happen if all paths store errors)
        return Err(scan_error);
    }

    Ok(())
}
