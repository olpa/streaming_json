# U8Pool

Use preallocated memory to store byte slices. The interface is stack-based, and there are `Vec` and `Map` iterators.

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

Memory layout for the example above:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              Buffer (1000 bytes)                        │
├─────────────────────────────────┬───────────────────────────────────────┤
│        Metadata Section         │            Data Section               │
│         (16 * 32 = 512)         │            (488 bytes)                │
├─────────────────────────────────┼───────────────────────────────────────┤
│ Slice 0: [0,4) len=4       →→→→→┼→→ nameAliceage30                      │
│ Slice 1: [4,9) len=5       →→→→→┼→→→→→→→┘    ↑  ↑                       │
│ Slice 2: [9,12) len=3      →→→→→┼→→→→→→→→→→→→┘  ↑                       │
│ Slice 3: [12,14) len=2     →→→→→┼→→→→→→→→→→→→→→→┘                       │
│ ... (28 unused slots)           │ ... (474 unused bytes)                │
└─────────────────────────────────┴───────────────────────────────────────┘
```