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

The converter handles all DynamoDB data types:

| DynamoDB Type | Standard JSON |
|---------------|---------------|
| `"S"` (String) | `string` |
| `"N"` (Number) | `number` |
| `"BOOL"` (Boolean) | `boolean` |
| `"NULL"` | `null` |
| `"M"` (Map) | `object` |
| `"L"` (List) | `array` |
| `"SS"` (String Set) | `array` |
| `"NS"` (Number Set) | `array` |
| `"BS"` (Binary Set) | `array` |
| `"B"` (Binary) | `string` (base64) |

### Example: Complex Nested Structure

**DynamoDB JSON:**
```json
{
  "Item": {
    "Publisher": {
      "M": {
        "Name": { "S": "Tech Publishing House" },
        "City": { "S": "New York" },
        "Founded": { "N": "1995" }
      }
    },
    "Tags": {
      "SS": ["programming", "technology", "reference"]
    },
    "Ratings": {
      "NS": ["4.5", "4.8", "5.0"]
    }
  }
}
```

**Standard JSON (after conversion):**
```json
{
  "Publisher": {
    "Name": "Tech Publishing House",
    "City": "New York",
    "Founded": 1995
  },
  "Tags": ["programming", "technology", "reference"],
  "Ratings": [4.5, 4.8, 5.0]
}
```

## Why Use This Tool?

- **Fast & Memory Efficient**: Streams data instead of loading everything into memory
- **Type-Safe**: Correctly converts DynamoDB's typed format to appropriate JSON types
- **Flexible**: Works with files or stdin/stdout for easy integration into pipelines
- **No Dependencies**: Small binary with minimal external dependencies

## Common Use Cases

### From DynamoDB to Standard JSON (from-ddb)

1. **Export DynamoDB data** to standard format for use with other tools
2. **Debug DynamoDB responses** by converting to readable JSON
3. **Batch process DynamoDB exports** using shell pipelines
4. **Transform data** before importing into other databases

### From Standard JSON to DynamoDB (to-ddb)

1. **Prepare data for DynamoDB import** from existing JSON files
2. **Generate test data** in DynamoDB format for testing
3. **Convert application JSON** to DynamoDB format for API calls
4. **Batch insert data** by converting JSON files to DynamoDB format

## Error Handling

The tool provides detailed error messages with context:

```
Conversion error: RJiter parsing error at position 42
  Error type: RJiter parsing error
  Position in input: 42 bytes
  Context: parsing nested map structure
  Details: Expected comma or closing brace
```

## Testing

Run the test suite with sample fixtures:

```bash
cargo test
```

Sample data files are available in the `fixture/` directory for testing:
- `book-dynamodb.json` / `book-normal.json`
- `all-types-dynamodb.json` / `all-types-normal.json`

## Limitations

### from-ddb mode
- Fully functional for all DynamoDB types

### to-ddb mode
- ✅ Fully functional (100% of tests passing)
- All basic types supported: strings, numbers, booleans, null
- All nested structures supported: arrays, objects, deeply nested combinations
- Sets (SS, NS, BS): Not currently supported, use Lists (L) instead
