use crate::StreamOp;
use crate::{rjiter::jiter::Peek, scan, Error as ScanError, RJiter, Result as ScanResult};
use crate::{BoxedAction, Name, Trigger};
use std::cell::RefCell;
use std::io::Write;

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
    };
    Err(ScanError::UnhandledPeek(peeked, rjiter.current_index()))
}

#[derive(Debug, PartialEq)]
enum IdtSequencePos {
    AtBeginning,
    InMiddle,
}

struct IdTransform<'a> {
    writer: &'a mut dyn Write,
    divider: IdtSequencePos,
}

impl<'a> IdTransform<'a> {
    fn new(writer: &'a mut dyn Write) -> Self {
        Self {
            writer,
            divider: IdtSequencePos::AtBeginning,
        }
    }
    fn get_writer_mut(&mut self) -> &mut dyn Write {
        self.writer
    }
}

fn on_atom(rjiter_cell: &RefCell<RJiter>, idt_cell: &RefCell<IdTransform>) -> StreamOp {
    let mut rjiter = rjiter_cell.borrow_mut();
    let mut idt = idt_cell.borrow_mut();

    match idt.divider {
        IdtSequencePos::AtBeginning => {
            idt.divider = IdtSequencePos::InMiddle;
        }
        IdtSequencePos::InMiddle => {
            if let Err(e) = idt.writer.write_all(b",") {
                return StreamOp::from(e);
            }
        }
    }

    match rjiter.peek() {
        Ok(peeked) => match copy_atom(peeked, &mut rjiter, idt.get_writer_mut()) {
            Ok(()) => StreamOp::ValueIsConsumed,
            Err(e) => StreamOp::from(e),
        },
        Err(e) => StreamOp::from(e),
    }
}

/// Do ID transform on the input JSON.
///
/// # Errors
///
/// This function will return an error if:
/// * The input JSON is malformed
/// * Nesting is too deep
/// * An IO error occurs while writing to the output
pub fn idtransform(rjiter_cell: &RefCell<RJiter>, writer: &mut dyn Write) -> ScanResult<()> {
    let idt = IdTransform::new(writer);
    let idt_cell = RefCell::new(idt);

    let trigger_atom: Trigger<BoxedAction<IdTransform>> =
        Trigger::new(Box::new(Name::new("#atom".to_string())), Box::new(on_atom));
    scan(&[trigger_atom], &[], &[], rjiter_cell, &idt_cell)
}
