use crate::{rjiter::jiter::Peek, Error as ScanError, RJiter, Result as ScanResult, scan};
use crate::{BoxedAction, Name, Trigger};
use crate::StreamOp;
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
        rjiter.write_long_bytes(writer)?;
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

struct IdTransform<'a> {
    writer: &'a mut dyn Write,
}

impl<'a> IdTransform<'a> {
    fn new(writer: &'a mut dyn Write) -> Self {
        Self { writer }
    }
    fn get_writer_mut(&mut self) -> &mut dyn Write {
        self.writer
    }
}

fn on_atom(rjiter_cell: &RefCell<RJiter>, idt_cell: &RefCell<IdTransform>) -> StreamOp {
    let mut rjiter = rjiter_cell.borrow_mut();
    let mut idt = idt_cell.borrow_mut();
    match rjiter.peek() {
        Ok(peeked) => match copy_atom(peeked, &mut rjiter, idt.get_writer_mut()) {
            Ok(_) => StreamOp::ValueIsConsumed,
            Err(e) => StreamOp::from(e),
        },
        Err(e) => StreamOp::from(e),
    }
}

pub fn idtransform(rjiter_cell: &RefCell<RJiter>, writer: &mut dyn Write) -> ScanResult<()> {
    let idt = IdTransform::new(writer);
    let idt_cell = RefCell::new(idt);

    let trigger_atom: Trigger<BoxedAction<IdTransform>> = Trigger::new(
        Box::new(Name::new("#atom".to_string())),
        Box::new(on_atom),
    );
    scan(&vec![trigger_atom], &vec![], &vec![], &rjiter_cell, &idt_cell)
}
