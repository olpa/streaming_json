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
    let obj = value
        .as_object()
        .context("Expected JSON object")?
        .clone();

    // Check if it has "Item" wrapper
    let obj = if obj.len() == 1 && obj.contains_key("Item") {
        obj.get("Item")
            .and_then(|v| v.as_object())
            .context("Expected Item to be an object")?
            .clone()
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
    let obj = value.as_object().context("Expected JSON object")?;

    // Marshall to DynamoDB format
    let mut result = Map::new();
    for (key, value) in obj {
        result.insert(key.clone(), marshall_value(value.clone())?);
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
    let obj = value.as_object().context("Expected DynamoDB type object")?;

    if obj.len() != 1 {
        anyhow::bail!("DynamoDB type object must have exactly one key");
    }

    let (type_key, type_value) = obj.iter().next().unwrap();

    match type_key.as_str() {
        "S" => Ok(type_value.clone()),
        "N" => {
            let s = type_value.as_str().context("N type must be string")?;
            // Try to parse as int first, then float
            if let Ok(i) = s.parse::<i64>() {
                Ok(Value::Number(i.into()))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::String(s.to_string())))
            } else {
                Ok(Value::String(s.to_string()))
            }
        }
        "BOOL" => Ok(type_value.clone()),
        "NULL" => Ok(Value::Null),
        "M" => {
            let map = type_value.as_object().context("M type must be object")?;
            let mut result = Map::new();
            for (k, v) in map {
                result.insert(k.clone(), unmarshall_value(v.clone())?);
            }
            Ok(Value::Object(result))
        }
        "L" => {
            let list = type_value.as_array().context("L type must be array")?;
            let mut result = Vec::new();
            for item in list {
                result.push(unmarshall_value(item.clone())?);
            }
            Ok(Value::Array(result))
        }
        "SS" => Ok(type_value.clone()),
        "NS" => {
            let arr = type_value.as_array().context("NS type must be array")?;
            let mut result = Vec::new();
            for item in arr {
                let s = item.as_str().context("NS items must be strings")?;
                if let Ok(i) = s.parse::<i64>() {
                    result.push(Value::Number(i.into()));
                } else if let Ok(f) = s.parse::<f64>() {
                    result.push(
                        serde_json::Number::from_f64(f)
                            .map(Value::Number)
                            .unwrap_or(Value::String(s.to_string())),
                    );
                } else {
                    result.push(Value::String(s.to_string()));
                }
            }
            Ok(Value::Array(result))
        }
        "BS" => Ok(type_value.clone()),
        "B" => Ok(type_value.clone()),
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
            // Check if it's a homogeneous array of strings or numbers
            if arr.is_empty() {
                let mut map = Map::new();
                map.insert("L".to_string(), Value::Array(vec![]));
                return Ok(Value::Object(map));
            }

            let all_strings = arr.iter().all(|v| v.is_string());
            let all_numbers = arr.iter().all(|v| v.is_number());

            if all_strings && arr.len() > 0 {
                // String Set
                let mut map = Map::new();
                map.insert("SS".to_string(), Value::Array(arr));
                Ok(Value::Object(map))
            } else if all_numbers && arr.len() > 0 {
                // Number Set - convert to strings
                let num_strings: Vec<Value> = arr
                    .iter()
                    .map(|v| Value::String(v.as_number().unwrap().to_string()))
                    .collect();
                let mut map = Map::new();
                map.insert("NS".to_string(), Value::Array(num_strings));
                Ok(Value::Object(map))
            } else {
                // List
                let mut items = Vec::new();
                for item in arr {
                    items.push(marshall_value(item)?);
                }
                let mut map = Map::new();
                map.insert("L".to_string(), Value::Array(items));
                Ok(Value::Object(map))
            }
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
