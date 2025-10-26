use clap::{Parser, ValueEnum};
use ddb_convert::convert_ddb_to_normal;
use std::io::{self, Read};

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
