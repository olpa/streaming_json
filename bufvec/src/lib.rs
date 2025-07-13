#![no_std]

//! `BufVec`: A zero-allocation vector implementation using client-provided buffers.
//!
//! `BufVec` provides vector, stack, and dictionary interfaces while using a single
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
//! `BufVec` is optimized for cache efficiency and minimal overhead:
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
//! bufvec = { version = "0.1", features = ["std"] }
//! ```
//!
//! # Dictionary Convention
//!
//! `BufVec` supports a dictionary interpretation where elements at even indices (0, 2, 4, ...)
//! are treated as keys and elements at odd indices (1, 3, 5, ...) are treated as values.
//! This allows the same data structure to be used as both a vector and a key-value store.
//!
//! ```
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Add key-value pairs using specialized methods
//! bufvec.add_key(b"name").unwrap();      // key at index 0
//! bufvec.add_value(b"Alice").unwrap();   // value at index 1
//! bufvec.add_key(b"age").unwrap();       // key at index 2
//! bufvec.add_value(b"30").unwrap();      // value at index 3
//!
//! // Specialized methods handle replacement logic
//! bufvec.add_key(b"country").unwrap();   // replaces "age" key since last element was a value
//! bufvec.add_value(b"USA").unwrap();     // adds normally since last element is now a key
//!
//! // Use dictionary interface
//! for (key, value) in bufvec.pairs() {
//!     match value {
//!         Some(v) => println!("{:?} = {:?}", key, v),
//!         None => println!("{:?} = <no value>", key),
//!     }
//! }
//!
//! // Check for unpaired keys
//! if bufvec.has_unpaired_key() {
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
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! bufvec.add_key(b"name").unwrap();
//! bufvec.add_key(b"username").unwrap();  // replaces "name" with "username"
//! bufvec.add_value(b"alice").unwrap();   // adds value for "username"
//! bufvec.add_value(b"alice123").unwrap(); // replaces "alice" with "alice123"
//!
//! assert_eq!(bufvec.len(), 2);
//! assert_eq!(bufvec.get(0), b"username");
//! assert_eq!(bufvec.get(1), b"alice123");
//! ```
//!
//! # Stack Interface
//!
//! `BufVec` supports stack operations through methods like `push()`, `pop()`, and `top()`:
//!
//! ```
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Push elements onto the stack
//! bufvec.push(b"first").unwrap();
//! bufvec.push(b"second").unwrap();
//! bufvec.push(b"third").unwrap();
//!
//! // Peek at the top element without removing it
//! assert_eq!(bufvec.top(), b"third");
//! assert_eq!(bufvec.len(), 3);
//!
//! // Pop elements in LIFO order
//! assert_eq!(bufvec.pop(), b"third");
//! assert_eq!(bufvec.pop(), b"second");
//! assert_eq!(bufvec.pop(), b"first");
//!
//! assert!(bufvec.is_empty());
//!
//! // Safe variants for error handling
//! assert!(bufvec.try_top().is_err());
//! assert!(bufvec.try_pop().is_err());
//! ```
//!
//! The stack interface maintains compatibility with vector operations:
//!
//! ```
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Mix stack and vector operations
//! bufvec.push(b"stack_data").unwrap();
//! bufvec.add(b"vector_data").unwrap();
//!
//! // Both interfaces work on the same underlying data
//! assert_eq!(bufvec.get(0), b"stack_data");
//! assert_eq!(bufvec.top(), b"vector_data");
//! ```
//!
//! # Iterator Support
//!
//! `BufVec` implements standard Rust iterator patterns:
//!
//! ```
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! bufvec.add(b"hello").unwrap();
//! bufvec.add(b"world").unwrap();
//!
//! // Iterate using for loop
//! for slice in &bufvec {
//!     println!("{:?}", slice);
//! }
//!
//! // Collect into Vec
//! let collected: Vec<_> = bufvec.into_iter().collect();
//! ```

mod core;
mod error;
mod iter;

// Re-export public types and traits
pub use core::BufVec;
pub use error::BufVecError;
pub use iter::{BufVecIter, BufVecPairIter};
