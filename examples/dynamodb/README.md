# DynamoDB JSON Converter

A fast, streaming JSON converter that transforms between DynamoDB's JSON format and standard JSON.

## What is DynamoDB JSON?

DynamoDB uses a special JSON format where each value is wrapped with a type descriptor. For example:

**DynamoDB JSON:**
```json
{
  "Item": {
    "Id": { "N": "103" },
    "Title": { "S": "Book 103 Title" },
    "Price": { "N": "2000" },
    "Available": { "BOOL": true }
  }
}
```

**Standard JSON:**
```json
{
  "Id": 103,
  "Title": "Book 103 Title",
  "Price": 2000,
  "Available": true
}
```

## Installation

Build the tool using Cargo:

```bash
cargo build --release
```

The binary will be available at `target/release/ddb_convert`.

## Usage

### Basic Syntax

```bash
ddb_convert <MODE> [OPTIONS]
```

### Conversion Modes

- `from-ddb` - Convert DynamoDB JSON to standard JSON
- `to-ddb` - Convert standard JSON to DynamoDB JSON

### Options

- `-i, --input <FILE>` - Input file (reads from stdin if not specified)
- `-o, --output <FILE>` - Output file (writes to stdout if not specified)
- `-p, --pretty` - Pretty-print the output JSON
- `--without-item` - Omit the top-level "Item" wrapper (only for `to-ddb` mode)

## Examples

### Convert from DynamoDB JSON to Standard JSON

**From file to file:**
```bash
ddb_convert from-ddb -i book-dynamodb.json -o book-normal.json
```

**Using stdin/stdout (useful for pipelines):**
```bash
cat book-dynamodb.json | ddb_convert from-ddb > book-normal.json
```

**Pretty-print the output:**
```bash
ddb_convert from-ddb -i book-dynamodb.json -p
```

**Stream directly from AWS CLI:**
```bash
aws dynamodb get-item --table-name Books --key '{"Id":{"N":"103"}}' \
  | ddb_convert from-ddb -p
```

### Convert from Standard JSON to DynamoDB JSON

**From file to file:**
```bash
ddb_convert to-ddb -i user.json -o user-dynamodb.json
```

**Using stdin/stdout:**
```bash
cat user.json | ddb_convert to-ddb > user-dynamodb.json
```

**Without the Item wrapper:**
```bash
# With Item wrapper (default)
ddb_convert to-ddb -i user.json
# Output: {"Item":{"name":{"S":"Alice"},"age":{"N":"30"}}}

# Without Item wrapper
ddb_convert to-ddb -i user.json --without-item
# Output: {"name":{"S":"Alice"},"age":{"N":"30"}}
```

**Prepare data for DynamoDB PutItem:**
```bash
# Convert your JSON and pipe to AWS CLI
cat mydata.json | ddb_convert to-ddb | \
  aws dynamodb put-item --table-name MyTable --item file:///dev/stdin
```

### Type Conversions

#### From DynamoDB to Standard JSON (`from-ddb`)

| DynamoDB Type | Standard JSON | Notes |
|---------------|---------------|-------|
| `"S"` (String) | `string` | |
| `"N"` (Number) | `number` | |
| `"BOOL"` (Boolean) | `boolean` | |
| `"NULL"` | `null` | |
| `"M"` (Map) | `object` | |
| `"L"` (List) | `array` | |
| `"SS"` (String Set) | `array` | ⚠️ Set becomes array (order/uniqueness lost) |
| `"NS"` (Number Set) | `array` | ⚠️ Set becomes array (order/uniqueness lost) |
| `"BS"` (Binary Set) | `array` | ⚠️ Set becomes array (order/uniqueness lost) |
| `"B"` (Binary) | `string` (base64) | base64 is ńot decoded |

**Note:** Standard JSON has no native "set" type, so DynamoDB sets are converted to arrays. The set semantics (unordered, unique values) are lost in the conversion.

#### From Standard JSON to DynamoDB (`to-ddb`)

| Standard JSON | DynamoDB Type | Notes |
|---------------|---------------|-------|
| `string` | `"S"` (String) | |
| `number` | `"N"` (Number) | |
| `boolean` | `"BOOL"` (Boolean) | |
| `null` | `"NULL"` | |
| `object` | `"M"` (Map) | |
| `array` | `"L"` (List) | Always creates Lists, not Sets |

**Note:** Arrays are always converted to DynamoDB Lists (`L`), not Sets. If you need Sets (SS, NS, BS), you must construct the DynamoDB JSON manually.

