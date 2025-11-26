use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use serde_json::{Map, Value};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ddb_convert_rust")]
#[command(about = "Convert between DynamoDB JSON and normal JSON formats", long_about = None)]
struct Cli {
    /// Conversion mode
    #[arg(value_enum)]
    mode: Mode,

    /// Input file (stdin if not specified)
    #[arg(short, long, value_name = "INPUT")]
    input: Option<PathBuf>,

    /// Output file (stdout if not specified)
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Pretty print output JSON
    #[arg(short, long)]
    pretty: bool,

    /// Omit top-level "Item" wrapper (only applies to to-ddb mode)
    #[arg(long)]
    without_item: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Mode {
    /// Convert DynamoDB JSON to normal JSON
    FromDdb,
    /// Convert normal JSON to DynamoDB JSON
    ToDdb,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Detect if input is JSONL
    let is_jsonl = if let Some(path) = &cli.input {
        detect_jsonl(path)?
    } else {
        false // stdin defaults to JSONL
    };

    // Open input
    let input: Box<dyn BufRead> = if let Some(path) = &cli.input {
        Box::new(BufReader::new(
            File::open(path).context("Failed to open input file")?,
        ))
    } else {
        Box::new(BufReader::new(io::stdin()))
    };

    // Open output
    let mut output: Box<dyn Write> = if let Some(path) = &cli.output {
        Box::new(BufWriter::new(
            File::create(path).context("Failed to create output file")?,
        ))
    } else {
        Box::new(BufWriter::new(io::stdout()))
    };

    if is_jsonl {
        process_jsonl(input, &mut output, cli.mode, cli.pretty, cli.without_item)?;
    } else {
        process_json(input, &mut output, cli.mode, cli.pretty, cli.without_item)?;
    }

    output.flush()?;
    Ok(())
}

fn detect_jsonl(path: &PathBuf) -> Result<bool> {
    // Check file extension first
    if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
        return Ok(true);
    }

    // For .json files, check if they contain multiple JSON objects
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut first_line = String::new();
    reader.read_line(&mut first_line)?;

    if first_line.trim().is_empty() {
        return Ok(false);
    }

    // Check if first line is valid JSON
    if serde_json::from_str::<Value>(&first_line).is_ok() {
        // Check if there's a second line
        let mut second_line = String::new();
        reader.read_line(&mut second_line)?;

        return Ok(!second_line.trim().is_empty());
    }

    Ok(false)
}

fn process_jsonl(
    input: Box<dyn BufRead>,
    output: &mut Box<dyn Write>,
    mode: Mode,
    pretty: bool,
    without_item: bool,
) -> Result<()> {
    for (line_num, line) in input.lines().enumerate() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let input_data: Value =
            serde_json::from_str(line).context(format!("Invalid JSON on line {}", line_num + 1))?;

        let output_data = match mode {
            Mode::FromDdb => from_dynamodb(input_data)?,
            Mode::ToDdb => to_dynamodb(input_data, !without_item)?,
        };

        if pretty {
            writeln!(output, "{}", serde_json::to_string_pretty(&output_data)?)?;
        } else {
            writeln!(output, "{}", serde_json::to_string(&output_data)?)?;
        }
    }

    Ok(())
}

fn process_json(
    mut input: Box<dyn BufRead>,
    output: &mut Box<dyn Write>,
    mode: Mode,
    pretty: bool,
    without_item: bool,
) -> Result<()> {
    let mut content = String::new();
    input.read_to_string(&mut content)?;

    let input_data: Value = serde_json::from_str(&content).context("Invalid JSON")?;

    let output_data = match mode {
        Mode::FromDdb => from_dynamodb(input_data)?,
        Mode::ToDdb => to_dynamodb(input_data, !without_item)?,
    };

    if pretty {
        writeln!(output, "{}", serde_json::to_string_pretty(&output_data)?)?;
    } else {
        writeln!(output, "{}", serde_json::to_string(&output_data)?)?;
    }

    Ok(())
}

fn from_dynamodb(value: Value) -> Result<Value> {
    let mut obj = match value {
        Value::Object(o) => o,
        _ => anyhow::bail!("Expected JSON object"),
    };

    // Check if it has "Item" wrapper - consume instead of clone
    let obj = if obj.len() == 1 && obj.contains_key("Item") {
        match obj.remove("Item") {
            Some(Value::Object(o)) => o,
            _ => anyhow::bail!("Expected Item to be an object"),
        }
    } else {
        obj
    };

    // Unmarshall DynamoDB format
    let mut result = Map::new();
    for (key, value) in obj {
        result.insert(key, unmarshall_value(value)?);
    }

    Ok(Value::Object(result))
}

fn to_dynamodb(value: Value, wrap_item: bool) -> Result<Value> {
    let obj = match value {
        Value::Object(o) => o,
        _ => anyhow::bail!("Expected JSON object"),
    };

    // Marshall to DynamoDB format
    let mut result = Map::new();
    for (key, value) in obj {
        result.insert(key, marshall_value(value)?);
    }

    if wrap_item {
        let mut wrapped = Map::new();
        wrapped.insert("Item".to_string(), Value::Object(result));
        Ok(Value::Object(wrapped))
    } else {
        Ok(Value::Object(result))
    }
}

fn unmarshall_value(value: Value) -> Result<Value> {
    let obj = match value {
        Value::Object(o) => o,
        _ => anyhow::bail!("Expected DynamoDB type object"),
    };

    if obj.len() != 1 {
        anyhow::bail!("DynamoDB type object must have exactly one key");
    }

    let (type_key, type_value) = obj.into_iter().next().unwrap();

    match type_key.as_str() {
        "S" => Ok(type_value),
        "N" => {
            let s = type_value.as_str().context("N type must be string")?;
            // Parse the number string into a JSON number
            let num: serde_json::Number = s.parse().context("Invalid number format")?;
            Ok(Value::Number(num))
        }
        "BOOL" => Ok(type_value),
        "NULL" => Ok(Value::Null),
        "M" => {
            let map = match type_value {
                Value::Object(o) => o,
                _ => anyhow::bail!("M type must be object"),
            };
            let mut result = Map::new();
            for (k, v) in map {
                result.insert(k, unmarshall_value(v)?);
            }
            Ok(Value::Object(result))
        }
        "L" => {
            let list = match type_value {
                Value::Array(a) => a,
                _ => anyhow::bail!("L type must be array"),
            };
            let mut result = Vec::new();
            for item in list {
                result.push(unmarshall_value(item)?);
            }
            Ok(Value::Array(result))
        }
        "SS" => Ok(type_value),
        "NS" => {
            let arr = type_value.as_array().context("NS type must be array")?;
            let mut result = Vec::new();
            for item in arr {
                let s = item.as_str().context("NS items must be strings")?;
                let num: serde_json::Number = s.parse().context("Invalid number format")?;
                result.push(Value::Number(num));
            }
            Ok(Value::Array(result))
        }
        "BS" => Ok(type_value),
        "B" => Ok(type_value),
        _ => anyhow::bail!("Unknown DynamoDB type: {}", type_key),
    }
}

fn marshall_value(value: Value) -> Result<Value> {
    match value {
        Value::Null => {
            let mut map = Map::new();
            map.insert("NULL".to_string(), Value::Bool(true));
            Ok(Value::Object(map))
        }
        Value::Bool(b) => {
            let mut map = Map::new();
            map.insert("BOOL".to_string(), Value::Bool(b));
            Ok(Value::Object(map))
        }
        Value::Number(n) => {
            let mut map = Map::new();
            map.insert("N".to_string(), Value::String(n.to_string()));
            Ok(Value::Object(map))
        }
        Value::String(s) => {
            let mut map = Map::new();
            map.insert("S".to_string(), Value::String(s));
            Ok(Value::Object(map))
        }
        Value::Array(arr) => {
            // Always use generic List type (L)
            let mut items = Vec::new();
            for item in arr {
                items.push(marshall_value(item)?);
            }
            let mut map = Map::new();
            map.insert("L".to_string(), Value::Array(items));
            Ok(Value::Object(map))
        }
        Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k, marshall_value(v)?);
            }
            let mut result = Map::new();
            result.insert("M".to_string(), Value::Object(map));
            Ok(Value::Object(result))
        }
    }
}
