# ddb_convert_rust

A high-performance Rust tool to convert between DynamoDB JSON format and normal JSON format.

## Features

- Convert DynamoDB JSON to normal JSON (`from-ddb`)
- Convert normal JSON to DynamoDB JSON (`to-ddb`)
- Support for JSONL (JSON Lines) format
- Automatic format detection
- Optional `Item` wrapper control
- Pretty print support
- Handles all DynamoDB types (S, N, BOOL, NULL, M, L, SS, NS, BS, B)

## Installation

Build the release binary:

```bash
cargo build --release
```

The binary will be at `target/release/ddb_convert_rust`.

## Usage

```bash
ddb_convert_rust [OPTIONS] <MODE>

Arguments:
  <MODE>  Conversion mode (from-ddb or to-ddb)

Options:
  -i, --input <INPUT>    Input file (stdin if not specified)
  -o, --output <OUTPUT>  Output file (stdout if not specified)
  -p, --pretty           Pretty print output JSON
  --without-item         Omit top-level "Item" wrapper (only for to-ddb mode)
  -h, --help             Print help
```

### Examples

Convert DynamoDB JSON to normal JSON:
```bash
ddb_convert_rust from-ddb -i input.json -o output.json
```

Convert normal JSON to DynamoDB JSON:
```bash
ddb_convert_rust to-ddb -i input.json -o output.json
```

With pretty printing:
```bash
ddb_convert_rust from-ddb -i input.json -p
```

Without Item wrapper:
```bash
ddb_convert_rust to-ddb -i input.json --without-item
```

## Format Support

### JSONL (JSON Lines)
The tool automatically detects JSONL format by:
- File extension (`.jsonl`)
- Content inspection (multiple JSON objects, one per line)

### DynamoDB Types

| DynamoDB Type | Normal JSON Type |
|---------------|------------------|
| S (String) | string |
| N (Number) | number |
| BOOL | boolean |
| NULL | null |
| M (Map) | object |
| L (List) | array |
| SS (String Set) | array of strings |
| NS (Number Set) | array of numbers |
| BS (Binary Set) | array of base64 strings |
| B (Binary) | base64 string |

## Testing

Run tests with the ddb-dump-small dataset:

```bash
cd ../ddb-dump-small
make from-ddb
make to-ddb
make check-eq
```

## Dependencies

- `clap` - CLI argument parsing
- `serde` / `serde_json` - JSON serialization with preserve_order feature
- `aws-sdk-dynamodb` - AWS SDK (for types reference)
- `anyhow` - Error handling

## License

Same as parent project.
