# BufVec

A zero-allocation vector implementation using client-provided buffers with vector, stack, and dictionary interfaces.

## Overview

`BufVec` provides a vector-like data structure that operates entirely within a client-provided buffer, performing zero heap allocations. It supports three interfaces:

- **Vector**: Standard indexed access with `add()`, `get()`, `len()`, etc.
- **Stack**: LIFO operations with `push()`, `pop()`, `top()`
- **Dictionary**: Key-value semantics where even indices are keys and odd indices are values

## Features

- **Zero heap allocations** - All data stored in client-provided buffer
- **Multiple interfaces** - Vector, stack, and dictionary views of the same data
- **Bounds checking** - All operations are bounds-checked with detailed error reporting
- **High performance** - O(1) operations for add, get, pop with optimized memory layout
- **Cache efficient** - Optimized descriptor access and sequential data allocation

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
bufvec = "0.1.0"
```

### Basic Vector Usage

```rust
use bufvec::BufVec;

let mut buffer = [0u8; 1000];
let mut bufvec = BufVec::with_default_max_slices(&mut buffer)?;

// Add elements
bufvec.add(b"hello")?;
bufvec.add(b"world")?;

// Access elements
assert_eq!(bufvec.get(0), b"hello");
assert_eq!(bufvec.get(1), b"world");
assert_eq!(bufvec.len(), 2);

// Iterate
for slice in &bufvec {
    println!("{:?}", std::str::from_utf8(slice).unwrap());
}
```

### Stack Interface

```rust
use bufvec::BufVec;

let mut buffer = [0u8; 1000];
let mut bufvec = BufVec::with_default_max_slices(&mut buffer)?;

// Push elements
bufvec.push(b"first")?;
bufvec.push(b"second")?;
bufvec.push(b"third")?;

// Peek at top
assert_eq!(bufvec.top(), b"third");

// Pop elements
assert_eq!(bufvec.pop(), Some(b"third"));
assert_eq!(bufvec.pop(), Some(b"second"));
assert_eq!(bufvec.len(), 1);
```

### Dictionary Interface

```rust
use bufvec::BufVec;

let mut buffer = [0u8; 1000];
let mut bufvec = BufVec::with_default_max_slices(&mut buffer)?;

// Add key-value pairs
bufvec.add_key(b"name")?;
bufvec.add_value(b"Alice")?;
bufvec.add_key(b"age")?;
bufvec.add_value(b"30")?;

// Iterate over pairs
for (key, value) in bufvec.pairs() {
    match value {
        Some(val) => println!("{:?} = {:?}", 
                             std::str::from_utf8(key).unwrap(),
                             std::str::from_utf8(val).unwrap()),
        None => println!("{:?} = <no value>", 
                        std::str::from_utf8(key).unwrap()),
    }
}

// Check dictionary properties
assert_eq!(bufvec.pairs_count(), 2);
assert!(!bufvec.has_unpaired_key());
```

## Buffer Management

### Buffer Layout

```
[metadata section][data section]
```

- **Metadata section**: Stores slice descriptors (start_offset, length) as 16-byte pairs
- **Data section**: Stores the actual slice data sequentially

### Choosing Buffer Size

Calculate buffer size as:
```
buffer_size = (max_slices * 16) + total_data_size
```

Example for 100 slices with average 50 bytes per slice:
```rust
let buffer_size = (100 * 16) + (100 * 50); // 6600 bytes
let mut buffer = vec![0u8; buffer_size];
let mut bufvec = BufVec::new(&mut buffer, 100)?;
```

### Memory Efficiency

```rust
let mut buffer = [0u8; 1000];
let mut bufvec = BufVec::with_default_max_slices(&mut buffer)?;

bufvec.add(b"data")?;

// Check memory usage
println!("Total capacity: {}", bufvec.buffer_capacity());
println!("Used bytes: {}", bufvec.used_bytes());
println!("Available bytes: {}", bufvec.available_bytes());
println!("Data used: {}", bufvec.data_used());
```

## Advanced Usage

### Mixed Interface Usage

```rust
use bufvec::BufVec;

let mut buffer = [0u8; 2000];
let mut bufvec = BufVec::new(&mut buffer, 50)?;

// Use as configuration parser
bufvec.add_key(b"host")?;
bufvec.add_value(b"localhost")?;
bufvec.add_key(b"port")?;
bufvec.add_value(b"8080")?;

// Add tags using vector interface
bufvec.add(b"production")?;
bufvec.add(b"web-server")?;

// Use stack for temporary processing
bufvec.push(b"processing")?;
let state = bufvec.top();
// ... do work ...
bufvec.pop(); // Remove temporary state (returns Option)

// Final verification using all interfaces
assert_eq!(bufvec.get(0), b"host");           // Vector access
assert_eq!(bufvec.pairs_count(), 3);          // Dictionary view
assert_eq!(bufvec.top(), b"web-server");      // Stack view
```

### Error Handling

BufVec uses the `thiserror` crate for comprehensive error handling with descriptive messages while maintaining `no_std` compatibility.

```rust
use bufvec::{BufVec, BufVecError};

let mut buffer = [0u8; 100]; // Small buffer
let mut bufvec = BufVec::new(&mut buffer, 5)?; // Few slices

// Handle buffer overflow with detailed error messages
match bufvec.add(&[0u8; 200]) { // Too large
    Ok(_) => println!("Added successfully"),
    Err(BufVecError::BufferOverflow { requested, available }) => {
        // Error includes descriptive Display message
        println!("Error: {}", BufVecError::BufferOverflow { requested, available }); // "Buffer overflow: requested 200 bytes, but only 84 bytes available"
        println!("Details: need {} bytes, only {} available", requested, available);
    }
    Err(e) => println!("Other error: {}", e),
}

// Handle slice limit exceeded
for i in 0..10 {
    match bufvec.add(format!("item_{}", i).as_bytes()) {
        Ok(_) => continue,
        Err(BufVecError::SliceLimitExceeded { max_slices }) => {
            println!("Error: {}", BufVecError::SliceLimitExceeded { max_slices }); // "Slice limit exceeded: maximum 5 slices allowed"
            println!("Reached slice limit of {}", max_slices);
            break;
        }
        Err(e) => println!("Other error: {}", e),
    }
}

// Handle index out of bounds
match bufvec.try_get(100) {
    Ok(data) => println!("Data: {:?}", data),
    Err(BufVecError::IndexOutOfBounds { index, length }) => {
        println!("Error: {}", BufVecError::IndexOutOfBounds { index, length }); // "Index out of bounds: index 100 is beyond vector length 3"
    }
    Err(e) => println!("Other error: {}", e),
}

// Using ? operator for error propagation
fn process_config(data: &[&[u8]]) -> Result<BufVec, BufVecError> {
    let mut buffer = [0u8; 1000];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer)?;
    
    for item in data {
        bufvec.add(item)?; // Automatically propagates any BufVecError
    }
    
    Ok(bufvec)
}

// Error handling with safe variants
let mut buffer = [0u8; 500];
let mut bufvec = BufVec::with_default_max_slices(&mut buffer)?;

// Safe variants return Option/Result instead of panicking
if let Ok(data) = bufvec.try_get(0) {
    println!("First element: {:?}", data);
} else {
    println!("No element at index 0");
}

if let Some(popped) = bufvec.pop() {
    println!("Popped element: {:?}", popped);
} else {
    println!("Stack is empty");
}

if let Ok(top) = bufvec.try_top() {
    println!("Top element: {:?}", top);
} else {
    println!("Stack is empty");
}
```

### Smart Dictionary Operations

```rust
let mut buffer = [0u8; 500];
let mut bufvec = BufVec::with_default_max_slices(&mut buffer)?;

// Smart key replacement
bufvec.add_key(b"name")?;
bufvec.add_key(b"username")?; // Replaces "name" with "username"
bufvec.add_value(b"alice")?;

// Smart value replacement  
bufvec.add_value(b"bob")?; // Replaces "alice" with "bob"

assert_eq!(bufvec.len(), 2); // Only 2 elements: key and value
assert_eq!(bufvec.get(0), b"username");
assert_eq!(bufvec.get(1), b"bob");
```

## Performance Characteristics

### Time Complexity

- **add()**, **push()**: O(1) - constant time insertion
- **get()**: O(1) - constant time access via descriptor lookup  
- **pop()**: O(1) - constant time removal, returns Option
- **clear()**: O(1) - resets metadata only
- **data_used()**: O(1) - optimized to use last slice position
- **Iterator operations**: O(n) - linear traversal

### Space Complexity

- **Memory overhead**: 16 bytes per slice (2 × usize for start/length)
- **Zero heap allocations** - all data stored in client-provided buffer
- **Optimal memory layout** with metadata section followed by data section

### Performance Guidelines

- Use larger buffers for better amortized performance
- Sequential access patterns are most efficient  
- Consider max_slices parameter based on expected element count
- Memory usage scales linearly with data size plus constant metadata overhead

## Real-World Examples

### JSON-like Data Parsing

```rust
use bufvec::BufVec;

let mut buffer = [0u8; 1000];
let mut bufvec = BufVec::new(&mut buffer, 20)?;

// Parse: {"name": "alice", "tags": ["dev", "rust"]}
bufvec.add_key(b"name")?;
bufvec.add_value(b"alice")?;

bufvec.add_key(b"tags")?;
bufvec.add_value(b"dev")?;
bufvec.add(b"rust")?; // Additional tag

// Process parsed data
for (key, value) in bufvec.pairs() {
    if key == b"name" {
        println!("Name: {:?}", std::str::from_utf8(value.unwrap()).unwrap());
    }
}

// Handle unpaired elements
if bufvec.has_unpaired_key() {
    let last_item = bufvec.get(bufvec.len() - 1);
    println!("Extra tag: {:?}", std::str::from_utf8(last_item).unwrap());
}
```

### Protocol Header Parsing

```rust
use bufvec::BufVec;

let mut buffer = [0u8; 800];
let mut bufvec = BufVec::new(&mut buffer, 15)?;

// Parse HTTP headers
bufvec.add_key(b"Content-Type")?;
bufvec.add_value(b"application/json")?;
bufvec.add_key(b"Content-Length")?;
bufvec.add_value(b"256")?;

// Add method and path
bufvec.add(b"POST")?;
bufvec.add(b"/api/users")?;

// Extract headers using dictionary interface
let headers: Vec<_> = bufvec.pairs().take(2).collect();
assert_eq!(headers[0].0, b"Content-Type");
assert_eq!(headers[0].1, Some(&b"application/json"[..]));

// Extract method and path using vector interface
assert_eq!(bufvec.get(4), b"POST");
assert_eq!(bufvec.get(5), b"/api/users");
```

## Error Types

BufVec uses `thiserror` for enhanced error handling with descriptive messages:

```rust
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum BufVecError {
    #[error("Buffer overflow: requested {requested} bytes, but only {available} bytes available")]
    BufferOverflow { requested: usize, available: usize },
    
    #[error("Index out of bounds: index {index} is beyond vector length {length}")]
    IndexOutOfBounds { index: usize, length: usize },
    
    #[error("Slice limit exceeded: maximum {max_slices} slices allowed")]
    SliceLimitExceeded { max_slices: usize },
    
    #[error("Zero-size buffer provided where data storage is required")]
    ZeroSizeBuffer,
    
    #[error("Invalid configuration: parameter '{parameter}' has invalid value {value}")]
    InvalidConfiguration { parameter: &'static str, value: usize },
}
```

Each error provides:
- **Structured data** for programmatic handling
- **Descriptive messages** via the `Display` trait for debugging
- **`no_std` compatibility** while maintaining rich error information

## Safety Guarantees

- All operations are bounds-checked
- No unsafe code in the public API
- Buffer integrity is maintained across all operations
- Panic-free operation when using `try_*` variants

## Safe vs Panicking APIs

Most operations have both panicking and safe variants:

```rust
// Panicking variants (for when you know bounds are correct)
let data = bufvec.get(0);
let top = bufvec.top();

// Safe variants (return Option/Result)
let data = bufvec.try_get(0)?;
let popped = bufvec.pop(); // Returns Option<&[u8]>
let top = bufvec.try_top()?;
```

## Contributing

Contributions are welcome! Please ensure all tests pass:

```bash
cargo test
cargo bench  # Run performance benchmarks
```

## License

[License information would go here]