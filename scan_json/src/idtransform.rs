use crate::matcher::Matcher;
use crate::StreamOp;
use crate::{
    rjiter::jiter::Peek, scan, scan::ContextFrame, Error as ScanError, RJiter, Result as ScanResult,
};
use crate::{BoxedAction, Trigger};
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

#[derive(Debug)]
struct IdtMatcherToHandler {
    is_top_level: bool,
}

struct IdTransform<'a> {
    writer: &'a mut dyn Write,
    divider: IdtSequencePos,
    matcher_to_handler: &'a RefCell<IdtMatcherToHandler>,
}

impl<'a> IdTransform<'a> {
    fn new(
        writer: &'a mut dyn Write,
        matcher_to_handler: &'a RefCell<IdtMatcherToHandler>,
    ) -> Self {
        Self {
            writer,
            divider: IdtSequencePos::AtBeginning,
            matcher_to_handler,
        }
    }

    fn get_writer_mut(&mut self) -> &mut dyn Write {
        self.writer
    }

    fn is_top_level(&self) -> bool {
        self.matcher_to_handler.borrow().is_top_level
    }

    fn write_divider(&mut self) -> Result<(), std::io::Error> {
        match self.divider {
            IdtSequencePos::AtBeginning => {
                self.divider = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::InMiddle => {
                let divider = if self.is_top_level() { b" " } else { b"," };
                self.writer.write_all(divider)?;
                self.divider = IdtSequencePos::InMiddle;
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
struct IdtMatcher<'a> {
    name: String,
    matcher_to_handler: &'a RefCell<IdtMatcherToHandler>,
}

impl<'a> IdtMatcher<'a> {
    fn new(name: String, matcher_to_handler: &'a RefCell<IdtMatcherToHandler>) -> Self {
        Self {
            name,
            matcher_to_handler,
        }
    }
}

impl<'a> Matcher for IdtMatcher<'a> {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool {
        if name != self.name {
            return false;
        }
        let mut matcher_to_handler = self.matcher_to_handler.borrow_mut();
        matcher_to_handler.is_top_level = context.len() < 2;
        true
    }
}

fn on_atom(rjiter_cell: &RefCell<RJiter>, idt_cell: &RefCell<IdTransform>) -> StreamOp {
    let mut rjiter = rjiter_cell.borrow_mut();
    let mut idt = idt_cell.borrow_mut();

    if let Err(e) = idt.write_divider() {
        return StreamOp::from(e);
    }

    match rjiter.peek() {
        Ok(peeked) => match copy_atom(peeked, &mut rjiter, idt.get_writer_mut()) {
            Ok(()) => StreamOp::ValueIsConsumed,
            Err(e) => StreamOp::from(e),
        },
        Err(e) => StreamOp::from(e),
    }
}

fn on_array(_rjiter_cell: &RefCell<RJiter>, idt_cell: &RefCell<IdTransform>) -> StreamOp {
    let mut idt = idt_cell.borrow_mut();

    if let Err(e) = idt.write_divider() {
        return StreamOp::from(e);
    }

    if let Err(e) = idt.writer.write_all(b"[") {
        return StreamOp::from(e);
    }
    idt.divider = IdtSequencePos::AtBeginning;
    StreamOp::None
}

///
/// # Errors
///
/// This function will return an error if:
/// * The input JSON is malformed
/// * Nesting is too deep
/// * An IO error occurs while writing to the output
pub fn idtransform(rjiter_cell: &RefCell<RJiter>, writer: &mut dyn Write) -> ScanResult<()> {
    let matcher_to_handler = RefCell::new(IdtMatcherToHandler { is_top_level: true });
    let idt = IdTransform::new(writer, &matcher_to_handler);
    let idt_cell = RefCell::new(idt);

    let trigger_atom: Trigger<BoxedAction<IdTransform>> = Trigger::new(
        Box::new(IdtMatcher::new("#atom".to_string(), &matcher_to_handler)),
        Box::new(on_atom),
    );

    let trigger_array: Trigger<BoxedAction<IdTransform>> = Trigger::new(
        Box::new(IdtMatcher::new("#array".to_string(), &matcher_to_handler)),
        Box::new(on_array),
    );

    // Have an intermediate result to avoid: borrowed value does not live long enough
    let result = scan(
        &[trigger_atom, trigger_array],
        &[],
        &[],
        rjiter_cell,
        &idt_cell,
    );
    #[allow(clippy::let_and_return)]
    result
}
