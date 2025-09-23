# `U8Pool`

Uses preallocated memory to store byte slices, optionally with a companion `Sized` object. The interface is stack-based, with `Vec` and `Map` iterators. The code is `no_std`, with `thiserror` as the only dependency.

## Example

```rust
use u8pool::U8Pool;

let mut buffer = [0u8; 1000];
let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

// Add key-value pairs
u8pool.push(b"name").unwrap();
u8pool.push(b"Alice").unwrap();
u8pool.push(b"age").unwrap();
u8pool.push(b"30").unwrap();

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
             std::str::from_utf8(value).unwrap());
}
// Output:
// "name" = "Alice"
// "age" = "30"
```

## Memory Layout

Memory layout for the example above:

```text
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

## Associated Values

In addition to storing byte slices, `U8Pool` supports associated values - structured data that can be paired with each byte slice. Associated values are stored directly in the buffer's metadata section before their corresponding data slice.

For example, you can store coordinates along with description strings:

```rust
use u8pool::U8Pool;

#[derive(Debug, PartialEq, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

let mut buffer = [0u8; 256];
let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

// Store a Point with associated data (returns references to stored values)
let point = Point { x: 42, y: 100 };
let (stored_point, stored_data) = pool.push_assoc(point, b"center point").unwrap();

// The returned references point to the same memory as get_assoc would return
assert_eq!(*stored_point, Point { x: 42, y: 100 });
assert_eq!(stored_data, b"center point");

// Retrieve both the Point and its data using get_assoc
let (retrieved_point, data) = pool.get_assoc::<Point>(0).unwrap();
assert_eq!(*retrieved_point, Point { x: 42, y: 100 });
assert_eq!(data, b"center point");
```

Associated values must implement the `Sized` trait and are stored using their memory representation. The library automatically ensures proper memory alignment for associated values by adding padding bytes when necessary.

## API Summary

**Construction:**

- `U8Pool::new(buffer: &mut [u8], max_slices: usize)` - Creates a pool with custom slice limit
- `U8Pool::with_default_max_slices(buffer: &mut [u8])` - Creates a pool with default limit (32 slices)

**Stack Operations:**

- `push(&mut self, data: &[u8]) -> Result<&[u8], U8PoolError>` - Adds a slice to the pool and returns a reference to the stored slice
- `pop(&mut self) -> Option<&[u8]>` - Removes and returns the last slice
- `get(&self, index: usize) -> Option<&[u8]>` - Accesses a slice by index
- `top(&self) -> Option<&[u8]>` - Returns the last slice without removing it. Can be implemented as `self.iter_rev().next()`
- `clear(&mut self)` - Removes all slices

**Associative Operations:**

- `push_assoc<T: Sized>(&mut self, assoc: T, data: &[u8]) -> Result<(&T, &[u8]), U8PoolError>` - Adds an associated value followed by a data slice and returns references to the stored values. Automatically handles memory alignment with padding as needed.
- `pop_assoc<T: Sized>(&mut self) -> Option<(&T, &[u8])>` - Removes and returns the last associated value and data slice
- `get_assoc<T: Sized>(&self, index: usize) -> Option<(&T, &[u8])>` - Accesses an associated value and data slice by index

**Information:**

- `len(&self) -> usize` - Returns the number of slices stored
- `is_empty(&self) -> bool` - Checks if the pool is empty

**Iteration:**

- `iter(&self)` - Returns a forward iterator over slices
- `iter_rev(&self)` - Returns a reverse iterator over slices
- `pairs(&self)` - Returns an iterator over key-value pairs (even/odd slices). If there is an odd number of slices, the last slice is ignored
- `iter_assoc<T: Sized>(&self)` - Returns a forward iterator over associated values and data slices
- `iter_assoc_rev<T: Sized>(&self)` - Returns a reverse iterator over associated values and data slices

All iterators implement the `Clone` trait, allowing you to create independent copies that can be advanced separately.

**Error Handling:**

All operations that can fail return `Result<T, U8PoolError>` with these error types:

- `InvalidInitialization` - Invalid buffer or `max_slices` parameter
- `SliceLimitExceeded` - Too many slices have been added
- `BufferOverflow` - Insufficient space for data
- `ValueTooLarge` - Slice position or length exceeds `u16::MAX`


## Colophon

License: MIT

Author: Oleg Parashchenko, olpa@ <https://uucode.com/>

Contact: via email or [Ailets Discord](https://discord.gg/HEBE3gv2)

`u8pool` is a part of the [streaming json](https://github.com/olpa/streaming_json) project, with other crates [rjiter](https://crates.io/crates/rjiter) and [scan_json](https://crates.io/crates/scan_json).
