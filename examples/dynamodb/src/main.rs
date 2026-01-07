//! `DynamoDB` JSON converter CLI tool

use clap::{Parser, ValueEnum};
use ddb_convert::{convert_ddb_to_normal, convert_normal_to_ddb, ConversionError};
use embedded_io_adapters::std::FromStd;
use std::io::{self, BufReader, BufWriter};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ConversionMode {
    /// Convert `DynamoDB` JSON to normal JSON
    FromDdb,
    /// Convert normal JSON to `DynamoDB` JSON
    ToDdb,
}

#[derive(Parser, Debug)]
#[command(name = "ddb_convert")]
#[command(version)]
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

    /// Omit top-level "Item" wrapper (only applies to to-ddb mode)
    ///
    /// When converting normal JSON to `DynamoDB` format, the output is wrapped
    /// in {"Item": {...}} by default. Use this flag to omit the wrapper.
    #[arg(long = "without-item", default_value_t = false)]
    without_item: bool,

    /// Do unbuffered reads and writes
    #[arg(long = "unbuffered", default_value_t = false)]
    unbuffered: bool,
}

/// Helper to create buffers and run conversion from `DynamoDB` JSON to normal JSON
fn convert_from_ddb<R: embedded_io::Read, W: embedded_io::Write>(
    input_reader: &mut R,
    output_writer: &mut W,
    pretty: bool,
) -> Result<(), ConversionError> {
    let mut rjiter_buffer = vec![0u8; 64 * 1024];
    let mut context_buffer = vec![0u8; 2048];
    convert_ddb_to_normal(
        input_reader,
        output_writer,
        &mut rjiter_buffer,
        &mut context_buffer,
        pretty,
        ddb_convert::ItemWrapperMode::AsWrapper,
    )
}

/// Helper to create buffers and run conversion from normal JSON to `DynamoDB` JSON
fn convert_to_ddb<R: embedded_io::Read, W: embedded_io::Write>(
    input_reader: &mut R,
    output_writer: &mut W,
    pretty: bool,
    with_item_wrapper: bool,
) -> Result<(), ConversionError> {
    let mut rjiter_buffer = vec![0u8; 64 * 1024];
    let mut context_buffer = vec![0u8; 2048];
    convert_normal_to_ddb(
        input_reader,
        output_writer,
        &mut rjiter_buffer,
        &mut context_buffer,
        pretty,
        with_item_wrapper,
    )
}

fn main() {
    let args = Args::parse();
    let buf_size = if args.unbuffered {
        1usize
    } else {
        32 * 1024usize
    };

    let result = match (args.mode, args.input, args.output) {
        (ConversionMode::FromDdb, Some(input_path), Some(output_path)) => {
            let input_file = open_input_file(&input_path);
            let output_file = create_output_file(&output_path);
            let mut input_reader = FromStd::new(BufReader::with_capacity(buf_size, input_file));
            let mut output_writer = FromStd::new(BufWriter::with_capacity(buf_size, output_file));
            convert_from_ddb(&mut input_reader, &mut output_writer, args.pretty)
        }
        (ConversionMode::FromDdb, Some(input_path), None) => {
            let input_file = open_input_file(&input_path);
            let mut input_reader = FromStd::new(BufReader::with_capacity(buf_size, input_file));
            let mut output_writer = FromStd::new(BufWriter::with_capacity(buf_size, io::stdout()));
            convert_from_ddb(&mut input_reader, &mut output_writer, args.pretty)
        }
        (ConversionMode::FromDdb, None, Some(output_path)) => {
            let output_file = create_output_file(&output_path);
            let mut input_reader = FromStd::new(BufReader::with_capacity(buf_size, io::stdin()));
            let mut output_writer = FromStd::new(BufWriter::with_capacity(buf_size, output_file));
            convert_from_ddb(&mut input_reader, &mut output_writer, args.pretty)
        }
        (ConversionMode::FromDdb, None, None) => {
            let mut input_reader = FromStd::new(BufReader::with_capacity(buf_size, io::stdin()));
            let mut output_writer = FromStd::new(BufWriter::with_capacity(buf_size, io::stdout()));
            convert_from_ddb(&mut input_reader, &mut output_writer, args.pretty)
        }
        (ConversionMode::ToDdb, Some(input_path), Some(output_path)) => {
            let input_file = open_input_file(&input_path);
            let output_file = create_output_file(&output_path);
            let mut input_reader = FromStd::new(BufReader::with_capacity(buf_size, input_file));
            let mut output_writer = FromStd::new(BufWriter::with_capacity(buf_size, output_file));
            convert_to_ddb(
                &mut input_reader,
                &mut output_writer,
                args.pretty,
                !args.without_item,
            )
        }
        (ConversionMode::ToDdb, Some(input_path), None) => {
            let input_file = open_input_file(&input_path);
            let mut input_reader = FromStd::new(BufReader::with_capacity(buf_size, input_file));
            let mut output_writer = FromStd::new(BufWriter::with_capacity(buf_size, io::stdout()));
            convert_to_ddb(
                &mut input_reader,
                &mut output_writer,
                args.pretty,
                !args.without_item,
            )
        }
        (ConversionMode::ToDdb, None, Some(output_path)) => {
            let output_file = create_output_file(&output_path);
            let mut input_reader = FromStd::new(BufReader::with_capacity(buf_size, io::stdin()));
            let mut output_writer = FromStd::new(BufWriter::with_capacity(buf_size, output_file));
            convert_to_ddb(
                &mut input_reader,
                &mut output_writer,
                args.pretty,
                !args.without_item,
            )
        }
        (ConversionMode::ToDdb, None, None) => {
            let mut input_reader = FromStd::new(BufReader::with_capacity(buf_size, io::stdin()));
            let mut output_writer = FromStd::new(BufWriter::with_capacity(buf_size, io::stdout()));
            convert_to_ddb(
                &mut input_reader,
                &mut output_writer,
                args.pretty,
                !args.without_item,
            )
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn open_input_file(path: &str) -> std::fs::File {
    std::fs::File::open(path).unwrap_or_else(|e| {
        eprintln!("Error opening input file '{path}': {e}");
        std::process::exit(1);
    })
}

fn create_output_file(path: &str) -> std::fs::File {
    std::fs::File::create(path).unwrap_or_else(|e| {
        eprintln!("Error creating output file '{path}': {e}");
        std::process::exit(1);
    })
}
