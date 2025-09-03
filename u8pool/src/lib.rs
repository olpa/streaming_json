#![no_std]

//! `U8Pool`: A zero-allocation vector implementation using client-provided buffers.
//!
//! `U8Pool` provides vector, stack, and dictionary interfaces while using a single
//! client-provided buffer for storage. All operations are bounds-checked and
//! no internal allocations are performed.
//!
//! This crate is `no_std` compatible and works in embedded and constrained environments.
//!
//! Buffer layout: [metadata section][data section]
//! Metadata section stores slice descriptors as (`start_offset`, length) pairs.
//!
//! # Performance Characteristics
//!
//! `U8Pool` is optimized for cache efficiency and minimal overhead:
//!
//! ## Time Complexity
//! - `add()`, `push()`: O(1) - constant time insertion
//! - `get()`: O(1) - constant time access via descriptor lookup
//! - `pop()`: O(1) - constant time removal
//! - `clear()`: O(1) - resets metadata only
//! - `data_used()`: O(1) - optimized to use last slice position
//! - Iterator operations: O(n) - linear traversal
//!
//! ## Space Complexity
//! - Memory overhead: 16 bytes per slice (2 × usize for start/length)
//! - Zero heap allocations - all data stored in client-provided buffer
//! - Optimal memory layout with metadata section followed by data section
//!
//! ## Cache Efficiency Optimizations
//! - Descriptor access uses single 16-byte slice operations for better cache locality
//! - Sequential data allocation for optimal cache line utilization
//! - Metadata stored contiguously at buffer start for efficient access patterns
//!
//! ## Performance Guidelines
//! - Use larger buffers for better amortized performance
//! - Sequential access patterns are most efficient
//! - Consider max_slices parameter based on expected element count
//! - Memory usage scales linearly with data size plus constant metadata overhead
//!
//! ## `no_std` Compatibility
//!
//! This crate is designed to work in `no_std` environments:
//! - No heap allocations - all data stored in provided buffers
//! - Uses only `core` library functionality
//! - No dependency on `std::error::Error` or `std::fmt::Display`
//! - Suitable for embedded systems and constrained environments
//!
//! Enable the optional `std` feature for additional functionality in std environments:
//! ```toml
//! [dependencies]
//! u8pool = { version = "0.1", features = ["std"] }
//! ```
//!
//! # Dictionary Convention
//!
//! `U8Pool` supports a dictionary interpretation where elements at even indices (0, 2, 4, ...)
//! are treated as keys and elements at odd indices (1, 3, 5, ...) are treated as values.
//! This allows the same data structure to be used as both a vector and a key-value store.
//!
//! ```
//! # use u8pool::U8Pool;
//! let mut buffer = [0u8; 600];
//! let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Add key-value pairs using specialized methods
//! u8pool.add_key(b"name").unwrap();      // key at index 0
//! u8pool.add_value(b"Alice").unwrap();   // value at index 1
//! u8pool.add_key(b"age").unwrap();       // key at index 2
//! u8pool.add_value(b"30").unwrap();      // value at index 3
//!
//! // Specialized methods handle replacement logic
//! u8pool.add_key(b"country").unwrap();   // replaces "age" key since last element was a value
//! u8pool.add_value(b"USA").unwrap();     // adds normally since last element is now a key
//!
//! // Use dictionary interface
//! for (key, value) in u8pool.pairs() {
//!     match value {
//!         Some(v) => println!("{:?} = {:?}", key, v),
//!         None => println!("{:?} = <no value>", key),
//!     }
//! }
//!
//! // Check for unpaired keys
//! if u8pool.has_unpaired_key() {
//!     println!("Last element is an unpaired key");
//! }
//! ```
//!
//! ## Replacement Semantics
//!
//! The specialized dictionary methods `add_key()` and `add_value()` implement smart replacement logic:
//!
//! - `add_key()`: If the last element is already a key, replaces it. Otherwise, adds normally.
//! - `add_value()`: If the last element is already a value, replaces it. Otherwise, adds normally.
//!
//! This allows for building dictionaries incrementally while correcting mistakes:
//!
//! ```
//! # use u8pool::U8Pool;
//! let mut buffer = [0u8; 600];
//! let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();
//!
//! u8pool.add_key(b"name").unwrap();
//! u8pool.add_key(b"username").unwrap();  // replaces "name" with "username"
//! u8pool.add_value(b"alice").unwrap();   // adds value for "username"
//! u8pool.add_value(b"alice123").unwrap(); // replaces "alice" with "alice123"
//!
//! assert_eq!(u8pool.len(), 2);
//! assert_eq!(u8pool.get(0), b"username");
//! assert_eq!(u8pool.get(1), b"alice123");
//! ```
//!
//! # Stack Interface
//!
//! `U8Pool` supports stack operations through methods like `push()`, `pop()`, and `top()`:
//!
//! ```
//! # use u8pool::U8Pool;
//! let mut buffer = [0u8; 600];
//! let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Push elements onto the stack
//! u8pool.push(b"first").unwrap();
//! u8pool.push(b"second").unwrap();
//! u8pool.push(b"third").unwrap();
//!
//! // Peek at the top element without removing it
//! assert_eq!(u8pool.top(), b"third");
//! assert_eq!(u8pool.len(), 3);
//!
//! // Pop elements in LIFO order
//! assert_eq!(u8pool.pop(), Some(&b"third"[..]));
//! assert_eq!(u8pool.pop(), Some(&b"second"[..]));
//! assert_eq!(u8pool.pop(), Some(&b"first"[..]));
//!
//! assert!(u8pool.is_empty());
//!
//! // Safe variants for error handling
//! assert!(u8pool.try_top().is_err());
//! assert!(u8pool.try_pop().is_err());
//! ```
//!
//! The stack interface maintains compatibility with vector operations:
//!
//! ```
//! # use u8pool::U8Pool;
//! let mut buffer = [0u8; 600];
//! let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Mix stack and vector operations
//! u8pool.push(b"stack_data").unwrap();
//! u8pool.add(b"vector_data").unwrap();
//!
//! // Both interfaces work on the same underlying data
//! assert_eq!(u8pool.get(0), b"stack_data");
//! assert_eq!(u8pool.top(), b"vector_data");
//! ```
//!
//! # Iterator Support
//!
//! `U8Pool` implements standard Rust iterator patterns:
//!
//! ```
//! # use u8pool::U8Pool;
//! let mut buffer = [0u8; 600];
//! let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();
//!
//! u8pool.add(b"hello").unwrap();
//! u8pool.add(b"world").unwrap();
//!
//! // Iterate using for loop
//! for slice in &u8pool {
//!     println!("{:?}", slice);
//! }
//!
//! // Collect into Vec
//! let collected: Vec<_> = u8pool.into_iter().collect();
//! ```

mod core;
mod error;
mod iter;

// Re-export public types and traits
pub use core::U8Pool;
pub use error::U8PoolError;
pub use iter::{U8PoolIter, U8PoolPairIter};
