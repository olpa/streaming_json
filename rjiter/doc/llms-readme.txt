RJiter - Streaming JSON Parser for Rust

## Overview

RJiter is a streaming JSON parser that processes large JSON files using a small buffer. It wraps the `Jiter` library and adds streaming capabilities by automatically reading more data when needed. RJiter is `no_std` compatible and uses `embedded-io` traits.

## Key Concepts

- **Streaming**: Processes JSON without loading the entire file into memory
- **Small Buffer**: Uses a configurable buffer size (e.g., 16 bytes) to control memory usage
- **Automatic Buffer Management**: Handles buffer shifts and reads transparently
- **Pass-through**: Supports writing long strings directly to output without buffering
- **No_std Compatible**: Works in embedded environments without standard library
- **Embedded-IO**: Uses `embedded-io` traits

## Basic Usage

```rust
use rjiter::{RJiter, jiter::{Peek, NumberInt}};

// Create a small buffer and reader
let mut buffer = [0u8; 16];
let mut reader = json_data.as_bytes();  // &[u8] implements embedded_io::Read
let mut rjiter = RJiter::new(&mut reader, &mut buffer);

// Parse JSON elements
rjiter.next_object()?;  // Returns Some("key") or None
rjiter.next_str()?;     // Returns string value
rjiter.next_int()?;     // Returns NumberInt
rjiter.finish()?;       // Ensures all JSON is consumed
```

## Crate Features

- **default**: No features enabled (no_std compatible)
- **std**: Enables std library features including Display trait and String allocation
- **display**: Enables Display implementations without requiring full std

## Main API Categories

### 1. Navigation Methods
- `next_object()` - Enter object, get first key
- `next_key()` - Get next key in object
- `next_array()` - Enter array, get first element type
- `array_step()` - Move to next array element
- `peek()` - Look at next value type without consuming

### 2. Value Reading Methods
- `next_str()` / `known_str()` - Read string values
- `next_int()` / `known_int()` - Read integer values
- `next_float()` / `known_float()` - Read float values
- `next_bool()` / `known_bool()` - Read boolean values
- `next_null()` / `known_null()` - Read null values
- `next_bytes()` / `known_bytes()` - Read raw bytes

### 3. Skipping and Lookahead Methods
- `next_skip()` / `known_skip()` - Skip any JSON value
- `known_skip_token()` - Skip specific text token
- `skip_n_bytes()` - Skip and consume n bytes
- `lookahead_while()` - Lookahead bytes while predicate matches
- `lookahead_n()` - Lookahead exactly n bytes

### 4. Long String Handling
- `write_long_bytes()` - Stream raw JSON string bytes to writer
- `write_long_str()` - Stream unescaped string content to writer

### 5. Utility Methods
- `finish()` - Verify all JSON consumed
- `current_index()` - Get current parser position
- `error_position()` - Get line/column for error reporting

## Memory Management

The buffer must be large enough to contain complete JSON elements. If a JSON element (like a string or number) is larger than the buffer, parsing will fail unless using the long string methods.

## Error Handling

All methods return `Result<T, Error>` where Error can be:
- IoError from reading the input stream (embedded-io compatible)
- JSON parsing errors from malformed JSON
- Buffer overflow errors when elements are too large
- Type mismatch errors when expected type doesn't match actual JSON type

## Advanced Features

### Token Skipping
For parsing JSON mixed with other text:
```rust
rjiter.known_skip_token(b"event:")?;  // Skip literal text
```

### Long String Pass-through
For strings longer than the buffer:
```rust
let mut output = Vec::new();
rjiter.write_long_str(&mut output)?;  // Streams string content
```

## Integration Notes

- Built on top of the fast Jiter parser
- Compatible with embedded_io::Read trait for input
- Uses embedded_io::Write trait for long string output
- Requires mutable borrows of buffer and reader
- No_std compatible by default
- Optional std feature provides additional convenience methods

For complete API documentation with all method signatures, see `llms-all.txt`.
For Jiter documentation, see `jiter/llms-readme.txt`.
