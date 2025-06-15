use crate::matcher::Matcher;
use crate::StreamOp;
use crate::{
    rjiter::jiter::Peek, scan, scan::ContextFrame, Error as ScanError, Name as NameMatcher, RJiter,
    Result as ScanResult,
};
use crate::{BoxedAction, BoxedEndAction, Trigger};
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

#[derive(Debug)]
enum IdtSequencePos {
    AtBeginning,
    InMiddle,
    AtBeginningKey(String),
    InMiddleKey(String),
    AfterKey,
}

struct IdTransform<'a> {
    writer: &'a mut dyn Write,
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
        println!("write_seqpos: seqpos: {:?}", self.seqpos); // FIXME
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
                self.writer.write_all(key.as_bytes())?;
                self.writer.write_all(b"\":")?;
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::InMiddleKey(key) => {
                self.writer.write_all(b",\"")?;
                self.writer.write_all(key.as_bytes())?;
                self.writer.write_all(b"\":")?;
                self.seqpos = IdtSequencePos::InMiddle;
                Ok(())
            }
            IdtSequencePos::AfterKey => Ok(()),
        }
    }
}

struct IdtMatcher<'a> {
    name: String,
    idt: &'a RefCell<IdTransform<'a>>,
}

impl<'a> std::fmt::Debug for IdtMatcher<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdtMatcher")
            .field("name", &self.name)
            .finish()
    }
}

impl<'a> IdtMatcher<'a> {
    fn new(name: String, idt: &'a RefCell<IdTransform<'a>>) -> Self {
        Self { name, idt }
    }
}

impl<'a> Matcher for IdtMatcher<'a> {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool {
        if name != self.name {
            return false;
        }
        let mut idt = self.idt.borrow_mut();
        idt.is_top_level = context.len() < 2;
        true
    }
}

struct IdtMatcherForKey<'a> {
    idt: &'a RefCell<IdTransform<'a>>,
}

impl<'a> std::fmt::Debug for IdtMatcherForKey<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdtMatcherForKey").finish()
    }
}

impl<'a> IdtMatcherForKey<'a> {
    fn new(idt: &'a RefCell<IdTransform<'a>>) -> Self {
        Self { idt }
    }
}

impl<'a> Matcher for IdtMatcherForKey<'a> {
    fn matches(&self, name: &str, _context: &[ContextFrame]) -> bool {
        let mut idt = self.idt.borrow_mut();
        println!("matches: name: {:?}, seqpos: {:?}", name, idt.seqpos); // FIXME
        idt.seqpos = match &idt.seqpos {
            IdtSequencePos::AtBeginning => IdtSequencePos::AtBeginningKey(name.to_string()),
            _ => IdtSequencePos::InMiddleKey(name.to_string()),
        };
        true
    }
}

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

    println!(
        "!!! on_atom: seqpos: {:?}, peeked: {:?}",
        idt.seqpos,
        rjiter.peek()
    ); // FIXME

    match rjiter.peek() {
        Ok(peeked) => match copy_atom(peeked, &mut rjiter, idt.get_writer_mut()) {
            Ok(()) => StreamOp::ValueIsConsumed,
            Err(e) => StreamOp::Error(Box::new(e)),
        },
        Err(e) => StreamOp::Error(Box::new(e)),
    }
}

fn on_struct(bytes: &[u8], idt_cell: &RefCell<IdTransform>) -> StreamOp {
    let mut idt = idt_cell.borrow_mut();

    println!("!!! on_struct: seqpos: {:?}", idt.seqpos); // FIXME

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

    let trigger_atom: Trigger<BoxedAction<IdTransform>> = Trigger::new(
        Box::new(IdtMatcher::new("#atom".to_string(), &idt_cell)),
        Box::new(on_atom),
    );

    let trigger_array: Trigger<BoxedAction<IdTransform>> = Trigger::new(
        Box::new(IdtMatcher::new("#array".to_string(), &idt_cell)),
        Box::new(on_array),
    );
    let trigger_array_end: Trigger<BoxedEndAction<IdTransform>> = Trigger::new(
        Box::new(NameMatcher::new("#array".to_string())),
        Box::new(on_array_end),
    );

    let trigger_object: Trigger<BoxedAction<IdTransform>> = Trigger::new(
        Box::new(IdtMatcher::new("#object".to_string(), &idt_cell)),
        Box::new(on_object),
    );
    let trigger_object_end: Trigger<BoxedEndAction<IdTransform>> = Trigger::new(
        Box::new(NameMatcher::new("#object".to_string())),
        Box::new(on_object_end),
    );

    let trigger_key: Trigger<BoxedAction<IdTransform>> =
        Trigger::new(Box::new(IdtMatcherForKey::new(&idt_cell)), Box::new(on_key));

    // Have an intermediate result to avoid: borrowed value does not live long enough
    let result = scan(
        &[trigger_atom, trigger_object, trigger_array, trigger_key],
        &[trigger_object_end, trigger_array_end],
        &[],
        rjiter_cell,
        &idt_cell,
    );
    #[allow(clippy::let_and_return)]
    result
}
