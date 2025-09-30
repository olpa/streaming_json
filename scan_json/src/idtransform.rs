//! Copy JSON input to output, retaining the original structure and collapsing whitespace.
//! The implementation of `idtransform` is an example of advanced use of the `scan` function.
//!
//! The code uses the `scan`'s parameter `baton_cell` of type `IdTransform` to:
//! - maintain state to properly write JSON, adding or not adding a comma, `IdtSequencePos`
//! - pass information from matchers to handlers, `IdtMatcherToHandler`
//!
//! Actually, there is no such thing as `IdtMatcherToHandler`, because doing "clean code"
//! and "good design" complicated the code too much. In the final implementation,
//! `IdTransform` took over the responsibility of the retired `IdtMatcherForKey`.
//!
//! Why pass the information at all? This is another trade-off.
//! The code uses a match-any-key matcher. The matched key should be printed to the output.
//! In the current implementation of `scan`:
//! - The matcher is not allowed to print anything.
//! - The handler doesn't know the key.
//!
//! How to print? The solution space is:
//! - Allow matchers to print.
//!   In this case, the return type of `scan` should be `Result`, not just `boolean`.
//! - Pass the key to the handler.
//!   In this case, the argument list of a handler should be extended, and `scan` should
//!   pass the context which was passed to the matcher once again, now to the handler.
//! - Pass the key from the matcher to the handler.
//!   In this case: 1) the matcher produces a side effect, 2) the printing is postponed
//!   to some unknown point in the future.
//!
use crate::StreamOp;
use crate::{
    rjiter::jiter::Peek, scan, Error as ScanError,
    Options, RJiter, Result as ScanResult,
    BoxedAction, BoxedEndAction,
};
use crate::stack::ContextIter;
use crate::matcher::StructuralPseudoname;
use std::cell::RefCell;
use std::io::Write;
use u8pool::U8Pool;

fn write_escaped_json_string(
    writer: &mut dyn Write,
    s: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for byte in s.bytes() {
        match byte {
            b'"' => writer.write_all(b"\\\"")?,
            b'\\' => writer.write_all(b"\\\\")?,
            b'\n' => writer.write_all(b"\\n")?,
            b'\r' => writer.write_all(b"\\r")?,
            b'\t' => writer.write_all(b"\\t")?,
            b => writer.write_all(&[b])?,
        }
    }
    Ok(())
}

/// Copy a JSON atom (string, number, boolean, or null) from the input to the output.
/// Advances the input iterator to the next token.
///
/// # Errors
///
/// This function will return an error if:
/// * The input JSON is malformed
/// * An IO error occurs while writing to the output
/// * An unexpected token type is encountered
pub fn copy_atom(peeked: Peek, rjiter: &mut RJiter, writer: &mut dyn Write) -> ScanResult<()> {
    if peeked == Peek::String {
        writer.write_all(b"\"")?;
        rjiter.write_long_bytes(writer)?;
        writer.write_all(b"\"")?;
        return Ok(());
    }
    if peeked == Peek::Null {
        rjiter.known_null()?;
        writer.write_all(b"null")?;
        return Ok(());
    }
    if peeked == Peek::True {
        rjiter.known_bool(peeked)?;
        writer.write_all(b"true")?;
        return Ok(());
    }
    if peeked == Peek::False {
        rjiter.known_bool(peeked)?;
        writer.write_all(b"false")?;
        return Ok(());
    }
    let maybe_number = rjiter.next_number_bytes();
    if let Ok(number) = maybe_number {
        writer.write_all(number)?;
        return Ok(());
    }
    Err(ScanError::UnhandledPeek(peeked, rjiter.current_index()))
}

// ---------------- State

#[derive(Debug)]
enum IdtSequencePos {
    AtBeginning,
    InMiddle,
    AtBeginningKey(String),
    InMiddleKey(String),
    AfterKey,
}

// Main transformer structure that maintains the state of the transformation process.
struct IdTransform<'a> {
    writer: &'a mut dyn Write,
    // `seqpos`+`is_top_level` could be the own type `IdtFromMatcherToHandler`
    seqpos: IdtSequencePos,
    is_top_level: bool,
}

impl<'a> IdTransform<'a> {
    fn new(writer: &'a mut dyn Write) -> Self {
        Self {
            writer,
            seqpos: IdtSequencePos::AtBeginning,
            is_top_level: true,
        }
    }

    fn get_writer_mut(&mut self) -> &mut dyn Write {
        self.writer
    }

    fn is_top_level(&self) -> bool {
        self.is_top_level
    }

    fn write_seqpos(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.seqpos {
            IdtSequencePos::AtBeginning => {
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::InMiddle => {
                let seqpos = if self.is_top_level() { b" " } else { b"," };
                self.writer.write_all(seqpos)?;
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::AtBeginningKey(key) => {
                self.writer.write_all(b"\"")?;
                write_escaped_json_string(self.writer, key)?;
                self.writer.write_all(b"\":")?;
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::InMiddleKey(key) => {
                self.writer.write_all(b",\"")?;
                write_escaped_json_string(self.writer, key)?;
                self.writer.write_all(b"\":")?;
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::AfterKey => Ok(()),
        }
    }
}

// ---------------- Matchers

/// Creates a `find_action` closure for `idtransform` that handles all JSON elements
///
/// # Arguments
/// * `idt_cell` - Reference cell containing the IdTransform state
///
/// # Returns
/// `find_action` parameter for the `scan` function
fn create_idtransform_find_action<'a>(
    idt_cell: &'a RefCell<IdTransform<'a>>
) -> impl Fn(StructuralPseudoname, ContextIter) -> Option<BoxedAction<IdTransform<'a>>> + 'a {
    move |structural_pseudoname: StructuralPseudoname, context: ContextIter| -> Option<BoxedAction<IdTransform<'a>>> {
        let context: Vec<&[u8]> = context.collect();
        match structural_pseudoname {
            StructuralPseudoname::Atom => {
                // Handle context for is_top_level
                let mut idt = idt_cell.borrow_mut();
                let context_count = context.len();
                idt.is_top_level = context_count < 2;
                Some(Box::new(on_atom))
            }
            StructuralPseudoname::Object => {
                let mut idt = idt_cell.borrow_mut();
                let context_count = context.len();
                idt.is_top_level = context_count < 2;
                Some(Box::new(on_object))
            }
            StructuralPseudoname::Array => {
                let mut idt = idt_cell.borrow_mut();
                let context_count = context.len();
                idt.is_top_level = context_count < 2;
                Some(Box::new(on_array))
            }
            StructuralPseudoname::None => {
                // Handle key matching - for object keys, the key name is in the context path
                if let Some(key_bytes) = context.first() {
                    if key_bytes != b"#top" && key_bytes != b"#array" {
                        let mut idt = idt_cell.borrow_mut();
                        let name_str = String::from_utf8_lossy(key_bytes).to_string();
                        idt.seqpos = match &idt.seqpos {
                            IdtSequencePos::AtBeginning => IdtSequencePos::AtBeginningKey(name_str),
                            _ => IdtSequencePos::InMiddleKey(name_str),
                        };
                        Some(Box::new(on_key))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
}

/// Creates a `find_end_action` closure for `idtransform` that handles end-of-structure events
///
/// # Returns
/// `find_end_action` parameter for the `scan` function
fn create_idtransform_find_end_action<'a>() -> impl Fn(StructuralPseudoname, ContextIter) -> Option<BoxedEndAction<IdTransform<'a>>> {
    |structural_pseudoname: StructuralPseudoname, _context: ContextIter| -> Option<BoxedEndAction<IdTransform<'a>>> {
        match structural_pseudoname {
            StructuralPseudoname::Object => Some(Box::new(on_object_end)),
            StructuralPseudoname::Array => Some(Box::new(on_array_end)),
            StructuralPseudoname::Atom | StructuralPseudoname::None => None,
        }
    }
}


// ---------------- Handlers

fn on_key(_rjiter_cell: &RefCell<RJiter>, idt_cell: &RefCell<IdTransform>) -> StreamOp {
    let mut idt = idt_cell.borrow_mut();

    if let Err(e) = idt.write_seqpos() {
        return StreamOp::Error(e);
    }
    idt.seqpos = IdtSequencePos::AfterKey;

    StreamOp::None
}

fn on_atom(rjiter_cell: &RefCell<RJiter>, idt_cell: &RefCell<IdTransform>) -> StreamOp {
    let mut rjiter = rjiter_cell.borrow_mut();
    let mut idt = idt_cell.borrow_mut();

    if let Err(e) = idt.write_seqpos() {
        return StreamOp::Error(e);
    }

    match rjiter.peek() {
        Ok(peeked) => match copy_atom(peeked, &mut rjiter, idt.get_writer_mut()) {
            Ok(()) => StreamOp::ValueIsConsumed,
            Err(e) => StreamOp::Error(Box::new(e)),
        },
        Err(e) => StreamOp::Error(Box::new(e)),
    }
}

// "Struct" means "array" or "object"
fn on_struct(bytes: &[u8], idt_cell: &RefCell<IdTransform>) -> StreamOp {
    let mut idt = idt_cell.borrow_mut();

    if let Err(e) = idt.write_seqpos() {
        return StreamOp::Error(e);
    }

    if let Err(e) = idt.writer.write_all(bytes) {
        return StreamOp::Error(Box::new(e));
    }
    idt.seqpos = IdtSequencePos::AtBeginning;
    StreamOp::None
}

fn on_struct_end(
    bytes: &[u8],
    idt_cell: &RefCell<IdTransform>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut idt = idt_cell.borrow_mut();
    idt.seqpos = IdtSequencePos::InMiddle;
    idt.writer.write_all(bytes)?;
    Ok(())
}

fn on_array(_rjiter_cell: &RefCell<RJiter>, idt_cell: &RefCell<IdTransform>) -> StreamOp {
    on_struct(b"[", idt_cell)
}

fn on_array_end(idt_cell: &RefCell<IdTransform>) -> Result<(), Box<dyn std::error::Error>> {
    on_struct_end(b"]", idt_cell)
}

fn on_object(_rjiter_cell: &RefCell<RJiter>, idt_cell: &RefCell<IdTransform>) -> StreamOp {
    on_struct(b"{", idt_cell)
}

fn on_object_end(idt_cell: &RefCell<IdTransform>) -> Result<(), Box<dyn std::error::Error>> {
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
/// * `rjiter_cell` - Reference cell containing the JSON iterator
/// * `writer` - Output writer for the transformed JSON
/// * `working_buffer` - Working buffer for context stack (see [`crate::scan`] for details)
///
/// # Errors
///
/// If `scan` fails (malformed json, nesting too deep, etc), return `scan`'s error.
/// Also, if an IO error occurs while writing to the output, return it.
///
pub fn idtransform(
    rjiter_cell: &RefCell<RJiter>,
    writer: &mut dyn Write,
    working_buffer: &mut U8Pool,
) -> ScanResult<()> {
    let idt = IdTransform::new(writer);
    let idt_cell = RefCell::new(idt);

    // Use helper functions to create the finder closures
    let find_action = create_idtransform_find_action(&idt_cell);
    let find_end_action = create_idtransform_find_end_action();

    // Use an intermediate result to avoid: borrowed value does not live long enough
    let result = scan(
        find_action,
        find_end_action,
        rjiter_cell,
        &idt_cell,
        working_buffer,
        &Options {
            sse_tokens: vec![],
            stop_early: true,
        },
    );
    #[allow(clippy::let_and_return)]
    result
}
