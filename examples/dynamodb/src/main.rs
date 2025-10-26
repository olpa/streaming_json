use clap::{Parser, ValueEnum};
use ddb_convert::{convert_ddb_to_normal, ConversionError};
use std::io;
use embedded_io_adapters::std::FromStd;

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

fn main() {
    let args = Args::parse();

    match args.mode {
        ConversionMode::DdbToNormal => {
            // Create buffers for streaming conversion
            let mut rjiter_buffer = vec![0u8; 4096];
            let mut context_buffer = vec![0u8; 2048];

            // Handle input - streaming from file or stdin
            let result = if let Some(input_path) = &args.input {
                // Stream from file
                let input_file = std::fs::File::open(input_path)
                    .expect("Failed to open input file");
                let mut input_reader = FromStd::new(input_file);

                // Handle output
                if let Some(output_path) = &args.output {
                    let output_file = std::fs::File::create(output_path)
                        .expect("Failed to create output file");
                    let mut output_writer = FromStd::new(output_file);
                    convert_ddb_to_normal(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
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
                    )
                }
            } else {
                // Stream from stdin
                let stdin = io::stdin();
                let mut input_reader = FromStd::new(stdin);

                if let Some(output_path) = &args.output {
                    let output_file = std::fs::File::create(output_path)
                        .expect("Failed to create output file");
                    let mut output_writer = FromStd::new(output_file);
                    convert_ddb_to_normal(
                        &mut input_reader,
                        &mut output_writer,
                        &mut rjiter_buffer,
                        &mut context_buffer,
                        args.pretty,
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
                    )
                }
            };

            if let Err(e) = result {
                eprintln!("Conversion error: {}", e);

                // Provide additional context based on error type
                match &e {
                    ConversionError::RJiterError { kind, position, context } => {
                        eprintln!("  Error type: RJiter parsing error");
                        eprintln!("  Position in input: {} bytes", position);
                        eprintln!("  Context: {}", context);
                        eprintln!("  Details: {:?}", kind);
                    }
                    ConversionError::IOError { kind, position, context } => {
                        eprintln!("  Error type: IO error");
                        eprintln!("  Position in input: {} bytes", position);
                        eprintln!("  Context: {}", context);
                        eprintln!("  Details: {:?}", kind);
                    }
                    ConversionError::ScanError(scan_err) => {
                        eprintln!("  Error type: JSON scanning error");
                        eprintln!("  Details: {:?}", scan_err);
                    }
                }

                std::process::exit(1);
            }
        }
        ConversionMode::NormalToDdb => {
            eprintln!("Normal to DDB conversion not yet implemented");
            std::process::exit(1);
        }
    }
}
