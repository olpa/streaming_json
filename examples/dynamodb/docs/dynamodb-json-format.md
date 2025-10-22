# AWS DynamoDB JSON Format - Official Documentation

> Source: AWS Official Documentation
> - https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Programming.LowLevelAPI.html
> - https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/S3DataExport.Output.html
> - https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/S3DataImport.Format.html

## Overview

DynamoDB employs JSON exclusively as a wire protocol rather than a storage mechanism. The system uses JavaScript Object Notation (JSON) as a transport format with name-value pairs defined in the format `name:value`. AWS SDKs handle JSON serialization and deserialization, allowing developers to focus on application logic while the protocol-level details remain abstracted.

## Data Type Descriptors

Every attribute in DynamoDB JSON requires a data type descriptor token that tells DynamoDB how to interpret each attribute. The complete list of descriptors:

| Descriptor | Type | Description |
|-----------|------|-------------|
| `S` | String | UTF-8 encoded string |
| `N` | Number | Numeric value (transmitted as string) |
| `B` | Binary | Base64-encoded binary data |
| `BOOL` | Boolean | Boolean value (true/false) |
| `NULL` | Null | Null value |
| `M` | Map | Nested document/object |
| `L` | List | Ordered collection of values |
| `SS` | String Set | Unordered set of strings |
| `NS` | Number Set | Unordered set of numbers |
| `BS` | Binary Set | Unordered set of binary values |

## Basic Item Structure

Each DynamoDB item in JSON format follows this pattern:

```json
{
  "Item": {
    "AttributeName": {
      "DataType": "value"
    }
  }
}
```

### Examples

**String Attribute:**
```json
{
  "Item": {
    "Title": {"S": "The Great Gatsby"}
  }
}
```

**Number Attribute:**
```json
{
  "Item": {
    "Year": {"N": "1925"}
  }
}
```

**Boolean Attribute:**
```json
{
  "Item": {
    "InStock": {"BOOL": true}
  }
}
```

**String Set:**
```json
{
  "Item": {
    "Authors": {"SS": ["F. Scott Fitzgerald", "Ernest Hemingway"]}
  }
}
```

**Map (Nested Object):**
```json
{
  "Item": {
    "Address": {
      "M": {
        "Street": {"S": "123 Main St"},
        "City": {"S": "New York"},
        "Zip": {"N": "10001"}
      }
    }
  }
}
```

**List:**
```json
{
  "Item": {
    "Tags": {
      "L": [
        {"S": "fiction"},
        {"S": "classic"},
        {"N": "1925"}
      ]
    }
  }
}
```

## Complete Item Example

```json
{
  "Item": {
    "Id": {"S": "book-001"},
    "Title": {"S": "The Great Gatsby"},
    "Author": {"S": "F. Scott Fitzgerald"},
    "Year": {"N": "1925"},
    "ISBN": {"S": "978-0-7432-7356-5"},
    "Price": {"N": "12.99"},
    "InStock": {"BOOL": true},
    "Categories": {"SS": ["Fiction", "Classic", "American Literature"]},
    "Ratings": {"NS": ["4.5", "4.8", "5.0"]},
    "Details": {
      "M": {
        "Publisher": {"S": "Scribner"},
        "Pages": {"N": "180"},
        "Language": {"S": "English"}
      }
    },
    "Reviews": {
      "L": [
        {"S": "A masterpiece"},
        {"S": "Timeless classic"}
      ]
    }
  }
}
```

## Import/Export Format

### Import Format (DynamoDB JSON)

For S3 imports, DynamoDB JSON files consist of multiple Item objects with newlines as item delimiters:

```json
{"Item": {"Id": {"S": "1"}, "Name": {"S": "Alice"}, "Age": {"N": "30"}}}
{"Item": {"Id": {"S": "2"}, "Name": {"S": "Bob"}, "Age": {"N": "25"}}}
{"Item": {"Id": {"S": "3"}, "Name": {"S": "Charlie"}, "Age": {"N": "35"}}}
```

**Important:** Newlines are used as item delimiters and should not be used within an item object.

### Export Format (Full Export)

For complete table exports, each item follows the standard marshalled JSON structure:

```json
{
  "Item": {
    "PrimaryKey": {"S": "key-value"},
    "Attribute1": {"N": "123"},
    "Attribute2": {"S": "value"}
  }
}
```

### Export Format (Incremental Export)

For incremental exports, items include metadata and operational context:

```json
{
  "Metadata": {
    "WriteTimestampMicros": 1234567890123456
  },
  "Keys": {
    "Id": {"S": "key-value"}
  },
  "NewImage": {
    "Id": {"S": "key-value"},
    "Name": {"S": "Updated Name"},
    "Status": {"S": "active"}
  },
  "OldImage": {
    "Id": {"S": "key-value"},
    "Name": {"S": "Old Name"},
    "Status": {"S": "inactive"}
  }
}
```

**Fields:**
- `Metadata`: Contains `WriteTimestampMicros` indicating when the item was modified
- `Keys`: The primary key attributes
- `NewImage`: Current item state after the change
- `OldImage`: Previous state (included when using "new and old images" view type)

## HTTP Request Structure

DynamoDB accepts HTTP(S) POST requests containing JSON payloads:

**Required Headers:**
- `Authorization`: AWS Signature Version 4 credentials
- `X-Amz-Target`: Specifies the operation (e.g., `DynamoDB_20120810.GetItem`)
- `Content-Type`: `application/x-amz-json-1.0`

**Request Body:**
```json
{
  "TableName": "MyTable",
  "Key": {
    "Id": {"S": "item-123"}
  }
}
```

## Special Considerations

### Numeric Handling

DynamoDB transmits numeric values as **strings** to prevent precision loss and unwanted type conversions. This approach preserves sorting semantics for values like "01", "2", and "03".

**Example:**
```json
{"Item": {"Price": {"N": "19.99"}}}
{"Item": {"Quantity": {"N": "42"}}}
```

### Binary Data

Binary attributes require **base64 encoding** before transmission. DynamoDB decodes the data upon receipt using RFC 4648 encoding standards.

**Example:**
```json
{"Item": {"Image": {"B": "iVBORw0KGgoAAAANSUhEUgAAAAUA..."}}}
```

### Null Values

Null values are explicitly typed:

```json
{"Item": {"OptionalField": {"NULL": true}}}
```

### Empty Sets

DynamoDB does not support empty sets. Sets must contain at least one element.

## File Format for Imports

When importing data from S3:
- Each line must be a complete, valid JSON object
- Use newlines as item delimiters
- Do not include newlines within item objects
- All items must include the table's primary key attributes
- File encoding should be UTF-8

## Operations Representation

In incremental exports, the structure indicates:
- **Insert**: Only `NewImage` present
- **Modify**: Both `NewImage` and `OldImage` present
- **Delete**: Only `OldImage` present

Items inserted and deleted within the same export window produce no output.

## Best Practices

1. **Use AWS SDKs**: Let SDKs handle JSON marshalling/unmarshalling automatically
2. **String Numbers**: Always transmit numbers as strings in the JSON format
3. **Base64 Encoding**: Properly encode binary data before transmission
4. **Set Validation**: Ensure sets are non-empty and contain unique values
5. **Type Consistency**: Maintain consistent data types for attributes across items
6. **Newline Delimiters**: Use proper newline delimiters for import files

## References

- [DynamoDB Low-Level API](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Programming.LowLevelAPI.html)
- [S3 Import Formats](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/S3DataImport.Format.html)
- [Table Export Output Format](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/S3DataExport.Output.html)
