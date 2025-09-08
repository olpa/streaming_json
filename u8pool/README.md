# U8Pool

Use preallocated memory to store byte slices. The interface is stack-based, with `Vec` and `Map` iterators. The code is `no_std`, with the only dependency `thiserror`.

## Example

```rust
use u8pool::U8Pool;

let mut buffer = [0u8; 1000];
let mut u8pool = U8Pool::with_default_max_slices(&mut buffer)?;

// Add key-value pairs
u8pool.push(b"name")?;
u8pool.push(b"Alice")?;
u8pool.push(b"age")?;
u8pool.push(b"30")?;

// Iterate over all elements
for element in &u8pool {
    println!("{:?}", std::str::from_utf8(element).unwrap());
}
// Output:
// "name"
// "Alice"
// "age"
// "30"

// Iterate over pairs
for (key, value) in u8pool.pairs() {
    println!("{:?} = {:?}", 
             std::str::from_utf8(key).unwrap(),
             std::str::from_utf8(value.unwrap()).unwrap());
}
// Output:
// "name" = "Alice"
// "age" = "30"
```

## Memory Layout

Memory layout for the example above:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              Buffer (1000 bytes)                        │
├─────────────────────────────────┬───────────────────────────────────────┤
│        Metadata Section         │            Data Section               │
│         (4 * 32 = 128)          │            (872 bytes)                │
├─────────────────────────────────┼───────────────────────────────────────┤
│ Slice 0: [0,4) len=4       →→→→→┼→→ nameAliceage30                      │
│ Slice 1: [4,9) len=5       →→→→→┼→→→→→→→┘    ↑  ↑                       │
│ Slice 2: [9,12) len=3      →→→→→┼→→→→→→→→→→→→┘  ↑                       │
│ Slice 3: [12,14) len=2     →→→→→┼→→→→→→→→→→→→→→→┘                       │
│ ... (28 unused slots)           │ ... (858 unused bytes)                │
└─────────────────────────────────┴───────────────────────────────────────┘
```

Each slice descriptor is stored as 4 bytes, with 2 bytes for the offset and 2 bytes for the length.

## API Summary

**Construction:**

- `U8Pool::new(buffer: &mut [u8], max_slices: usize)` - Create pool with custom slice limit
- `U8Pool::with_default_max_slices(buffer: &mut [u8])` - Create pool with default limit (32 slices)

**Stack Operations:**

- `push(&mut self, data: &[u8])` - Add slice to the pool
- `pop(&mut self) -> Option<&[u8]>` - Remove and return last slice
- `get(&self, index: usize) -> Option<&[u8]>` - Access slice by index
- `clear(&mut self)` - Remove all slices

**Information:**

- `len(&self) -> usize` - Number of slices stored
- `is_empty(&self) -> bool` - Check if pool is empty

**Iteration:**

- `iter(&self)` - Forward iterator over slices
- `iter_rev(&self)` - Reverse iterator over slices  
- `pairs(&self)` - Iterator over key-value pairs (even/odd slices)

**Error Handling:**

All operations that can fail return `Result<T, U8PoolError>` with these error types:

- `InvalidInitialization` - Invalid buffer or max_slices parameter
- `SliceLimitExceeded` - Too many slices added
- `BufferOverflow` - Insufficient space for data
- `ValueTooLarge` - Slice position or length exceeds u16::MAX


## Colophon

License: MIT

Author: Oleg Parashchenko, olpa@ <https://uucode.com/>

Contact: via email or [Ailets Discord](https://discord.gg/HEBE3gv2)

`u8pool` is a part of the [streaming json](https://github.com/olpa/streaming_json) project, with other crates [rjiter](https://crates.io/crates/rjiter) and [scan_json](https://crates.io/crates/scan_json).
