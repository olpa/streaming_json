# DynamoDB JSON Format Fixtures

This directory contains authoritative examples of JSON data in both **normal JSON** format and **DynamoDB JSON** format, based on official AWS documentation.

## File Pairs

Each dataset is provided in two formats for easy comparison:

### 1. Book Example (Single Item)
- `book-normal.json` - Standard JSON format
- `book-dynamodb.json` - DynamoDB JSON format with type descriptors

**Data types demonstrated:**
- String (S)
- Number (N)
- Boolean (BOOL)
- Null (NULL)
- String Set (SS)
- Number Set (NS)
- List (L)
- Map (M)
- Binary (B)

### 2. Users Example (Multiple Items)
- `users-normal.json` - Standard JSON array format
- `users-dynamodb.jsonl` - DynamoDB JSON Lines format (newline-delimited)

**Data types demonstrated:**
- All scalar types (S, N, BOOL)
- String Sets (SS) for roles and tags
- Number Sets (NS) for favorite numbers
- Nested Maps (M) for profile and preferences
- Null values in nested objects

### 3. Products Example (Multiple Items)
- `products-normal.json` - Standard JSON array format
- `products-dynamodb.jsonl` - DynamoDB JSON Lines format

**Data types demonstrated:**
- Binary Set (BS) for images
- String Set (SS) for tags
- Number Set (NS) for ratings
- List (L) for reviews
- Map (M) for specifications
- Null values

### 4. All Types Example (Comprehensive)
- `all-types-normal.json` - Standard JSON with all data types
- `all-types-dynamodb.json` - DynamoDB JSON with all type descriptors

**Complete demonstration of:**
- All 10 DynamoDB data types
- Nested structures
- Mixed-type lists
- Deep nesting
- Edge cases

## DynamoDB Data Type Descriptors

| Descriptor | Type | Example |
|-----------|------|---------|
| `S` | String | `{"S": "Hello"}` |
| `N` | Number | `{"N": "42"}` |
| `B` | Binary | `{"B": "dGVzdA=="}` |
| `BOOL` | Boolean | `{"BOOL": true}` |
| `NULL` | Null | `{"NULL": true}` |
| `M` | Map | `{"M": {"key": {"S": "value"}}}` |
| `L` | List | `{"L": [{"S": "a"}, {"N": "1"}]}` |
| `SS` | String Set | `{"SS": ["a", "b", "c"]}` |
| `NS` | Number Set | `{"NS": ["1", "2", "3"]}` |
| `BS` | Binary Set | `{"BS": ["dGVzdA==", "ZGF0YQ=="]}` |

## Format Differences

### Normal JSON
```json
{
  "name": "Alice",
  "age": 28,
  "active": true,
  "tags": ["admin", "user"]
}
```

### DynamoDB JSON
```json
{
  "Item": {
    "name": {"S": "Alice"},
    "age": {"N": "28"},
    "active": {"BOOL": true},
    "tags": {"SS": ["admin", "user"]}
  }
}
```

## Key Observations

### Numbers as Strings
In DynamoDB JSON, numbers are transmitted as strings to preserve precision:
```json
{"price": {"N": "29.99"}}
```

### Sets vs Lists
- **Sets** (SS, NS, BS): Unordered, unique values of the same type
- **Lists** (L): Ordered, can contain mixed types

**Normal JSON:**
```json
{"tags": ["a", "b"]}
```

**DynamoDB as String Set:**
```json
{"tags": {"SS": ["a", "b"]}}
```

**DynamoDB as List:**
```json
{"tags": {"L": [{"S": "a"}, {"S": "b"}]}}
```

### Nested Structures
Maps and Lists can be deeply nested:
```json
{
  "profile": {
    "M": {
      "location": {
        "M": {
          "city": {"S": "New York"}
        }
      }
    }
  }
}
```

### Binary Data
Binary data must be base64-encoded:
```json
{"image": {"B": "aVZCT1J3MEtHZ29BQUFBTg=="}}
```

### Null Values
Null is explicitly typed in DynamoDB:
```json
{"description": {"NULL": true}}
```

## File Formats

### JSON Files (.json)
- Pretty-printed for readability
- Single item or array of items
- Standard JSON structure

### JSON Lines Files (.jsonl)
- One complete JSON object per line
- No comma separators between objects
- Standard format for DynamoDB imports/exports
- Each line is a valid JSON object

## Usage Examples

### Loading Normal JSON
```javascript
const data = require('./book-normal.json');
console.log(data.Title); // "Book 103 Title"
```

### Loading DynamoDB JSON
```javascript
const dynamoData = require('./book-dynamodb.json');
console.log(dynamoData.Item.Title.S); // "Book 103 Title"
```

### Processing JSON Lines
```javascript
const fs = require('fs');
const lines = fs.readFileSync('./users-dynamodb.jsonl', 'utf8')
  .split('\n')
  .filter(line => line.trim())
  .map(line => JSON.parse(line));
```

## Sources

These fixtures are based on:
- [AWS DynamoDB Low-Level API Documentation](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Programming.LowLevelAPI.html)
- [S3 Import Formats for DynamoDB](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/S3DataImport.Format.html)
- [DynamoDB Table Export Output Format](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/S3DataExport.Output.html)
- [Supported Data Types and Naming Rules](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.NamingRulesDataTypes.html)

## Testing and Validation

These fixtures can be used for:
- Testing JSON parsers and serializers
- Validating DynamoDB marshalling/unmarshalling
- Learning DynamoDB data type conversions
- Unit testing AWS SDK integrations
- Benchmarking JSON processing performance
