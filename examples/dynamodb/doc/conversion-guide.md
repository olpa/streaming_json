# DynamoDB JSON Conversion Guide

This guide demonstrates the conversion between Normal JSON and DynamoDB JSON format with side-by-side examples.

## Quick Reference

### Scalar Types

| Type | Normal JSON | DynamoDB JSON |
|------|-------------|---------------|
| String | `"Hello"` | `{"S": "Hello"}` |
| Number | `42` | `{"N": "42"}` |
| Number | `3.14` | `{"N": "3.14"}` |
| Boolean | `true` | `{"BOOL": true}` |
| Boolean | `false` | `{"BOOL": false}` |
| Null | `null` | `{"NULL": true}` |

### Collection Types

| Type | Normal JSON | DynamoDB JSON |
|------|-------------|---------------|
| String Set | `["a", "b"]` | `{"SS": ["a", "b"]}` |
| Number Set | `[1, 2, 3]` | `{"NS": ["1", "2", "3"]}` |
| Binary Set | `["Zmlyc3Q="]` | `{"BS": ["Zmlyc3Q="]}` |
| List | `["a", 1, true]` | `{"L": [{"S": "a"}, {"N": "1"}, {"BOOL": true}]}` |
| Map | `{"key": "val"}` | `{"M": {"key": {"S": "val"}}}` |
| Binary | `"dGVzdA=="` | `{"B": "dGVzdA=="}` |

## Detailed Examples

### Example 1: Simple Object

**Normal JSON:**
```json
{
  "id": "user-001",
  "name": "Alice",
  "age": 28,
  "active": true
}
```

**DynamoDB JSON:**
```json
{
  "Item": {
    "id": {"S": "user-001"},
    "name": {"S": "Alice"},
    "age": {"N": "28"},
    "active": {"BOOL": true}
  }
}
```

### Example 2: Arrays (Sets vs Lists)

**Normal JSON with Homogeneous Array:**
```json
{
  "tags": ["admin", "editor", "user"]
}
```

**DynamoDB JSON as String Set (SS):**
```json
{
  "Item": {
    "tags": {"SS": ["admin", "editor", "user"]}
  }
}
```

**DynamoDB JSON as List (L):**
```json
{
  "Item": {
    "tags": {
      "L": [
        {"S": "admin"},
        {"S": "editor"},
        {"S": "user"}
      ]
    }
  }
}
```

**Key Difference:**
- **String Set (SS)**: Unordered, unique values, more efficient
- **List (L)**: Ordered, can have duplicates, can mix types

### Example 3: Nested Objects

**Normal JSON:**
```json
{
  "profile": {
    "firstName": "Alice",
    "lastName": "Smith",
    "location": {
      "city": "San Francisco",
      "zipCode": "94102"
    }
  }
}
```

**DynamoDB JSON:**
```json
{
  "Item": {
    "profile": {
      "M": {
        "firstName": {"S": "Alice"},
        "lastName": {"S": "Smith"},
        "location": {
          "M": {
            "city": {"S": "San Francisco"},
            "zipCode": {"S": "94102"}
          }
        }
      }
    }
  }
}
```

### Example 4: Mixed-Type Lists

**Normal JSON:**
```json
{
  "mixedData": [
    "string",
    42,
    true,
    {"nested": "object"}
  ]
}
```

**DynamoDB JSON:**
```json
{
  "Item": {
    "mixedData": {
      "L": [
        {"S": "string"},
        {"N": "42"},
        {"BOOL": true},
        {"M": {"nested": {"S": "object"}}}
      ]
    }
  }
}
```

### Example 5: Number Sets

**Normal JSON:**
```json
{
  "ratings": [4.5, 4.8, 5.0]
}
```

**DynamoDB JSON:**
```json
{
  "Item": {
    "ratings": {"NS": ["4.5", "4.8", "5.0"]}
  }
}
```

**Note:** Numbers in DynamoDB JSON are always strings to preserve precision.

### Example 6: Null Values

**Normal JSON:**
```json
{
  "description": null,
  "notes": "Some text"
}
```

**DynamoDB JSON:**
```json
{
  "Item": {
    "description": {"NULL": true},
    "notes": {"S": "Some text"}
  }
}
```

### Example 7: Binary Data

**Normal JSON:**
```json
{
  "image": "aVZCT1J3MEtHZ29BQUFBTg==",
  "thumbnail": "dGh1bWJuYWlsZGF0YQ=="
}
```

**DynamoDB JSON:**
```json
{
  "Item": {
    "image": {"B": "aVZCT1J3MEtHZ29BQUFBTg=="},
    "thumbnail": {"B": "dGh1bWJuYWlsZGF0YQ=="}
  }
}
```

**Binary Set:**
```json
{
  "Item": {
    "images": {
      "BS": [
        "aVZCT1J3MEtHZ29BQUFBTg==",
        "dGh1bWJuYWlsZGF0YQ=="
      ]
    }
  }
}
```

### Example 8: Complete Real-World Object

**Normal JSON:**
```json
{
  "userId": "user-001",
  "username": "alice_smith",
  "email": "alice@example.com",
  "age": 28,
  "isActive": true,
  "roles": ["admin", "editor"],
  "favoriteNumbers": [7, 42, 100],
  "profile": {
    "firstName": "Alice",
    "lastName": "Smith",
    "location": {
      "city": "San Francisco",
      "state": "CA"
    }
  },
  "preferences": {
    "theme": "dark",
    "notifications": true
  },
  "bio": null,
  "score": 95.7
}
```

**DynamoDB JSON:**
```json
{
  "Item": {
    "userId": {"S": "user-001"},
    "username": {"S": "alice_smith"},
    "email": {"S": "alice@example.com"},
    "age": {"N": "28"},
    "isActive": {"BOOL": true},
    "roles": {"SS": ["admin", "editor"]},
    "favoriteNumbers": {"NS": ["7", "42", "100"]},
    "profile": {
      "M": {
        "firstName": {"S": "Alice"},
        "lastName": {"S": "Smith"},
        "location": {
          "M": {
            "city": {"S": "San Francisco"},
            "state": {"S": "CA"}
          }
        }
      }
    },
    "preferences": {
      "M": {
        "theme": {"S": "dark"},
        "notifications": {"BOOL": true}
      }
    },
    "bio": {"NULL": true},
    "score": {"N": "95.7"}
  }
}
```

## Conversion Rules

### 1. Every Value Needs a Type Descriptor
In DynamoDB JSON, every value must be wrapped in an object with a type descriptor.

### 2. Numbers Are Strings
```json
// Normal JSON
{"age": 28}

// DynamoDB JSON
{"age": {"N": "28"}}
```

### 3. Arrays Can Be Sets or Lists

**Use Sets (SS, NS, BS) when:**
- All elements are the same type
- Order doesn't matter
- Values should be unique
- Better performance needed

**Use Lists (L) when:**
- Elements have different types
- Order matters
- Duplicates are allowed
- Need to preserve exact structure

### 4. Objects Become Maps
```json
// Normal JSON
{"address": {"city": "NYC"}}

// DynamoDB JSON
{"address": {"M": {"city": {"S": "NYC"}}}}
```

### 5. Null Must Be Explicit
```json
// Normal JSON
{"field": null}

// DynamoDB JSON
{"field": {"NULL": true}}
```

### 6. Binary Data Must Be Base64
All binary data must be base64-encoded strings in both formats.

## Import/Export Format

### Single Item Format
Used for single operations (GetItem, PutItem):
```json
{
  "Item": {
    "id": {"S": "001"},
    "name": {"S": "Alice"}
  }
}
```

### JSON Lines Format
Used for batch imports/exports (one item per line):
```jsonl
{"Item":{"id":{"S":"001"},"name":{"S":"Alice"}}}
{"Item":{"id":{"S":"002"},"name":{"S":"Bob"}}}
{"Item":{"id":{"S":"003"},"name":{"S":"Charlie"}}}
```

**Important:**
- Each line is a complete JSON object
- No comma separators between lines
- No newlines within objects
- Commonly used for S3 imports/exports

## Common Pitfalls

### ❌ Wrong: Numbers as Numbers
```json
{"age": {"N": 28}}  // Wrong!
```

### ✅ Correct: Numbers as Strings
```json
{"age": {"N": "28"}}  // Correct
```

### ❌ Wrong: Empty Sets
```json
{"tags": {"SS": []}}  // Wrong! Sets cannot be empty
```

### ✅ Correct: Omit Attribute or Use List
```json
// Option 1: Don't include the attribute
{"name": {"S": "Alice"}}

// Option 2: Use an empty list instead
{"tags": {"L": []}}
```

### ❌ Wrong: Missing Type Descriptors
```json
{"Item": {"name": "Alice"}}  // Wrong!
```

### ✅ Correct: All Values Have Types
```json
{"Item": {"name": {"S": "Alice"}}}  // Correct
```

## Validation Checklist

- [ ] All values have type descriptors (S, N, BOOL, etc.)
- [ ] Numbers are quoted as strings
- [ ] Sets are non-empty and homogeneous
- [ ] Binary data is base64-encoded
- [ ] Null values use `{"NULL": true}`
- [ ] Nested objects use Map type (M)
- [ ] Mixed-type arrays use List type (L)
- [ ] Import files use newline delimiters
- [ ] No newlines within item objects

## Tools for Conversion

### AWS SDK JavaScript
```javascript
const { marshall, unmarshall } = require('@aws-sdk/util-dynamodb');

// Normal JSON to DynamoDB JSON
const dynamoJson = marshall({name: "Alice", age: 28});

// DynamoDB JSON to Normal JSON
const normalJson = unmarshall(dynamoJson);
```

### AWS SDK Python (boto3)
```python
from boto3.dynamodb.types import TypeSerializer, TypeDeserializer

serializer = TypeSerializer()
deserializer = TypeDeserializer()

# Normal to DynamoDB
dynamo_json = {k: serializer.serialize(v) for k, v in normal_json.items()}

# DynamoDB to Normal
normal_json = {k: deserializer.deserialize(v) for k, v in dynamo_json.items()}
```

## See Also

- [DynamoDB JSON Format Documentation](../docs/dynamodb-json-format.md)
- [Fixture Examples](./README.md)
- [AWS DynamoDB Developer Guide](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/)
