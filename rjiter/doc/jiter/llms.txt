jiter - Fast Iterable JSON Parser

## Overview
jiter is a high-performance JSON parsing library for Rust that provides three main interfaces for different use cases. It's designed to be significantly faster than standard JSON parsers while maintaining memory efficiency.

## Core Interfaces

### 1. JsonValue - Simple JSON Representation
The `JsonValue` enum represents JSON data as Rust types:

```rust
use jiter::JsonValue;

let json_data = r#"{"name": "John", "age": 30}"#;
let value = JsonValue::parse(json_data.as_bytes(), true).unwrap();
```

### 2. Jiter - Streaming Iterator
The `Jiter` struct provides streaming JSON parsing with precise control:

```rust
use jiter::{Jiter, NumberInt, Peek};

let mut jiter = Jiter::new(json_data.as_bytes()).with_allow_inf_nan();
assert_eq!(jiter.next_object().unwrap(), Some("name"));
assert_eq!(jiter.next_str().unwrap(), "John");
```

### 3. PythonParse - Python Integration
The `PythonParse` trait enables parsing JSON directly into Python objects (requires "python" feature).

## Key Features

### Performance
- 2-4x faster than serde_json in benchmarks
- Memory efficient streaming parsing
- SIMD optimizations for aarch64

### Data Type Support
- Integers (including big integers with "num-bigint" feature)
- Floating point numbers with configurable precision
- Unicode strings with caching
- Arrays and objects with lazy evaluation

### Configuration Options
- `allow_inf_nan`: Allow infinite and NaN float values
- `partial_mode`: Support for parsing incomplete JSON
- `string_cache_mode`: Control string interning behavior

## Common Usage Patterns

### Parsing Known Structure
```rust
// When you know the JSON structure
let mut jiter = Jiter::new(data);
let name = jiter.next_object()?.and_then(|_| jiter.next_str().ok());
```

### Dynamic Parsing
```rust
// When structure is unknown
let value = JsonValue::parse(data, true)?;
match value {
    JsonValue::Object(obj) => { /* handle object */ }
    JsonValue::Array(arr) => { /* handle array */ }
    _ => { /* handle other types */ }
}
```

### Streaming Large Files
```rust
// For memory-efficient parsing of large JSON
let mut jiter = Jiter::new(large_json_data);
while let Some(key) = jiter.next_key()? {
    // Process each key-value pair without loading entire structure
}
```

## Error Handling
All parsing operations return `Result<T, JiterError>` with detailed error information including line position for debugging.

## Features
- Default: `["num-bigint"]` - Support for arbitrary precision integers
- `"python"` - Python integration via PyO3
- `"num-bigint"` - Big integer support

For complete API documentation with all methods and types, see: llms-all.txt
