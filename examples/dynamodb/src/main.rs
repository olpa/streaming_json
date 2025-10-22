use clap::{Parser, ValueEnum};
use embedded_io::Write as IoWrite;
use rjiter::jiter::Peek;
use rjiter::RJiter;
use scan_json::{iter_match, scan, Action, EndAction, Options, StreamOp};
use scan_json::matcher::StructuralPseudoname;
use scan_json::stack::ContextIter;
use std::cell::RefCell;
use std::io::{self, Read};
use u8pool::U8Pool;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ConversionMode {
    /// Convert DynamoDB JSON to normal JSON
    DdbToNormal,
    /// Convert normal JSON to DynamoDB JSON
    NormalToDdb,
}

#[derive(Parser, Debug)]
#[command(name = "ddb_convert")]
#[command(about = "Convert between DynamoDB JSON and normal JSON formats", long_about = None)]
struct Args {
    /// Conversion mode
    #[arg(value_enum)]
    mode: ConversionMode,

    /// Input file (stdin if not specified)
    #[arg(short, long)]
    input: Option<String>,

    /// Output file (stdout if not specified)
    #[arg(short, long)]
    output: Option<String>,

    /// Pretty print output JSON
    #[arg(short, long, default_value_t = false)]
    pretty: bool,
}

struct DdbConverter<'a, 'workbuf, W: IoWrite> {
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

fn on_type_descriptor_object<R: embedded_io::Read, W: IoWrite>(_rjiter: &mut RJiter<R>, _baton: DdbBaton<'_, '_, W>) -> StreamOp {
    // Type descriptor object - just let scan handle it, don't write anything yet
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

fn find_action<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Match "Item" key at root
    if iter_match(|| [b"Item", b"#top"], structural, context.clone()) {
        return Some(on_item_begin);
    }

    // Match fields inside Item - these will be type descriptor objects
    if structural == StructuralPseudoname::None {
        let mut ctx = context.clone();
        if let Some(field) = ctx.next() {
            if field != b"#top" && field != b"#array" {
                if let Some(parent) = ctx.next() {
                    if parent == b"Item" {
                        // This is a field key inside Item
                        // Store field name and prepare to write it
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

    // Match type descriptor objects - these are Objects that are values of fields in Item
    if structural == StructuralPseudoname::Object {
        let mut ctx = context.clone();
        if let Some(field_name) = ctx.next() {
            if field_name != b"#top" && field_name != b"#array" {
                if let Some(parent) = ctx.next() {
                    if parent == b"Item" {
                        // This is a type descriptor object - just let scan handle it
                        return Some(on_type_descriptor_object);
                    }
                }
            }
        }
    }

    // Match type descriptor keys (N, S, SS, etc.) - these are keys inside the type descriptor object
    if structural == StructuralPseudoname::None {
        let mut ctx = context.clone();
        if let Some(type_key) = ctx.next() {
            if let Some(field_name) = ctx.next() {
                if field_name != b"#top" && field_name != b"#array" {
                    if let Some(parent) = ctx.next() {
                        if parent == b"Item" {
                            // This is a type descriptor key like "N", "S", etc.
                            // Match specific types and return appropriate handlers
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

    // Match end of type descriptor objects - any object that ends and isn't Item
    if structural == StructuralPseudoname::Object {
        let mut ctx = context.clone();
        if let Some(first) = ctx.next() {
            // If first element is "Item", it's the Item object ending
            if first == b"Item" {
                // Already handled above
            } else if first != b"#top" && first != b"#array" {
                // This is some other object - likely a type descriptor
                return Some(on_type_descriptor_end);
            }
        }
    }

    // Match end of SS, NS, BS, L arrays
    if structural == StructuralPseudoname::Array {
        let mut ctx = context.clone();
        if let Some(type_key) = ctx.next() {
            if let Some(_field_name) = ctx.next() {
                if let Some(_parent) = ctx.next() {
                    match type_key {
                        b"SS" | b"BS" | b"NS" | b"L" => return Some(on_set_or_list_end),
                        _ => {}
                    }
                }
            }
        }
    }

    // Match end of M (Map) - but this conflicts with type descriptor end above
    // Actually M objects are inside type descriptors, so they have a different context
    // Let me check the context more carefully...

    None
}

fn convert_ddb_to_normal(input: &str, pretty: bool) -> Result<String, String> {
    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 4096];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Use a large buffer for output - embedded_io::Write is implemented for &mut [u8]
    let mut output_buffer = vec![0u8; 1024 * 1024]; // 1MB buffer
    let mut output_slice = output_buffer.as_mut_slice();
    let converter = DdbConverter::new(&mut output_slice, pretty);
    let baton = RefCell::new(converter);

    let mut working_buffer = [0u8; 2048];
    let mut context = U8Pool::new(&mut working_buffer, 32)
        .map_err(|e| format!("Failed to create context pool: {:?}", e))?;

    scan(
        find_action,
        find_end_action,
        &mut rjiter,
        &baton,
        &mut context,
        &Options::new(),
    )
    .map_err(|e| format!("Scan error: {:?}", e))?;

    // Calculate how many bytes were written
    let bytes_written = 1024 * 1024 - output_slice.len();
    String::from_utf8(output_buffer[..bytes_written].to_vec())
        .map_err(|e| format!("UTF-8 error: {}", e))
}

fn main() {
    let args = Args::parse();

    // Read input
    let input_data = match &args.input {
        Some(path) => {
            std::fs::read_to_string(path).expect("Failed to read input file")
        }
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .expect("Failed to read from stdin");
            buffer
        }
    };

    // Perform conversion
    let output_data = match args.mode {
        ConversionMode::DdbToNormal => {
            match convert_ddb_to_normal(&input_data, args.pretty) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("Conversion error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        ConversionMode::NormalToDdb => {
            eprintln!("Normal to DDB conversion not yet implemented");
            std::process::exit(1);
        }
    };

    // Write output
    match &args.output {
        Some(path) => {
            std::fs::write(path, output_data).expect("Failed to write output file");
        }
        None => {
            print!("{}", output_data);
        }
    }
}
