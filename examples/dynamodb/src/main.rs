//! `DynamoDB` JSON converter CLI tool

use clap::{Parser, ValueEnum};
use ddb_convert::convert_ddb_to_normal;
use embedded_io_adapters::std::FromStd;
use std::io;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ConversionMode {
    /// Convert `DynamoDB` JSON to normal JSON
    FromDdb,
    /// Convert normal JSON to `DynamoDB` JSON
    ToDdb,
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

    /// Omit top-level "Item" wrapper (only applies to to-ddb mode)
    ///
    /// When converting normal JSON to `DynamoDB` format, the output is wrapped
    /// in {"Item": {...}} by default. Use this flag to omit the wrapper.
    #[arg(long = "without-item", default_value_t = false)]
    without_item: bool,
}

fn main() {
    let args = Args::parse();

    match args.mode {
        ConversionMode::FromDdb => {
            // Create buffers for streaming conversion
            let mut rjiter_buffer = vec![0u8; 4096];
            let mut context_buffer = vec![0u8; 2048];

            // Handle input - streaming from file or stdin
            let result = if let Some(input_path) = &args.input {
                // Stream from file
                let input_file = match std::fs::File::open(input_path) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Error opening input file '{input_path}': {e}");
                        std::process::exit(1);
                    }
                };
                let mut input_reader = FromStd::new(input_file);

                // Handle output
                if let Some(output_path) = &args.output {
                    let output_file = match std::fs::File::create(output_path) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Error creating output file '{output_path}': {e}");
                            std::process::exit(1);
                        }
                    };
                    let mut output_writer = FromStd::new(output_file);
                    convert_ddb_to_normal(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
                        ddb_convert::ItemWrapperMode::AsWrapper,
                    )
                } else {
                    let stdout = io::stdout();
                    let mut output_writer = FromStd::new(stdout);
                    convert_ddb_to_normal(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
                        ddb_convert::ItemWrapperMode::AsWrapper,
                    )
                }
            } else {
                // Stream from stdin
                let stdin = io::stdin();
                let mut input_reader = FromStd::new(stdin);

                if let Some(output_path) = &args.output {
                    let output_file = match std::fs::File::create(output_path) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Error creating output file '{output_path}': {e}");
                            std::process::exit(1);
                        }
                    };
                    let mut output_writer = FromStd::new(output_file);
                    convert_ddb_to_normal(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
                        ddb_convert::ItemWrapperMode::AsWrapper,
                    )
                } else {
                    let stdout = io::stdout();
                    let mut output_writer = FromStd::new(stdout);
                    convert_ddb_to_normal(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
                        ddb_convert::ItemWrapperMode::AsWrapper,
                    )
                }
            };

            if let Err(e) = result {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        ConversionMode::ToDdb => {
            // Create buffers for streaming conversion
            let mut rjiter_buffer = vec![0u8; 4096];
            let mut context_buffer = vec![0u8; 2048];

            // Determine whether to use Item wrapper (default is true, unless --without-item is specified)
            let with_item_wrapper = !args.without_item;

            // Handle input - streaming from file or stdin
            let result = if let Some(input_path) = &args.input {
                // Stream from file
                let input_file = match std::fs::File::open(input_path) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Error opening input file '{input_path}': {e}");
                        std::process::exit(1);
                    }
                };
                let mut input_reader = FromStd::new(input_file);

                // Handle output
                if let Some(output_path) = &args.output {
                    let output_file = match std::fs::File::create(output_path) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Error creating output file '{output_path}': {e}");
                            std::process::exit(1);
                        }
                    };
                    let mut output_writer = FromStd::new(output_file);
                    ddb_convert::convert_normal_to_ddb(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
                        with_item_wrapper,
                    )
                } else {
                    let stdout = io::stdout();
                    let mut output_writer = FromStd::new(stdout);
                    ddb_convert::convert_normal_to_ddb(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
                        with_item_wrapper,
                    )
                }
            } else {
                // Stream from stdin
                let stdin = io::stdin();
                let mut input_reader = FromStd::new(stdin);

                if let Some(output_path) = &args.output {
                    let output_file = match std::fs::File::create(output_path) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Error creating output file '{output_path}': {e}");
                            std::process::exit(1);
                        }
                    };
                    let mut output_writer = FromStd::new(output_file);
                    ddb_convert::convert_normal_to_ddb(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
                        with_item_wrapper,
                    )
                } else {
                    let stdout = io::stdout();
                    let mut output_writer = FromStd::new(stdout);
                    ddb_convert::convert_normal_to_ddb(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
                        with_item_wrapper,
                    )
                }
            };

            if let Err(e) = result {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
}
