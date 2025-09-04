#![no_std]

//! `U8Pool`: A zero-allocation vector implementation using client-provided buffers.
//!
//! `U8Pool` provides vector, stack, and dictionary interfaces while using a single
//! client-provided buffer for storage. All operations are bounds-checked and
//! no internal allocations are performed.
//!
//! This crate is `no_std` compatible.
//!
//! Buffer layout: [metadata section][data section]
//! Metadata section stores slice descriptors as (`start_offset`, length) pairs.
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
//! // Add key-value pairs using push
//! u8pool.push(b"name").unwrap();      // key at index 0
//! u8pool.push(b"Alice").unwrap();     // value at index 1
//! u8pool.push(b"age").unwrap();       // key at index 2
//! u8pool.push(b"30").unwrap();        // value at index 3
//!
//! // Use dictionary interface with pairs iterator
//! for (key, value) in u8pool.pairs() {
//!     match value {
//!         Some(v) => println!("{:?} = {:?}", key, v),
//!         None => println!("{:?} = <no value>", key),
//!     }
//! }
//! ```
//!

//! # Stack Interface
//!
//! `U8Pool` supports stack operations through methods like `push()` and `pop()`:
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
//! // Pop elements in LIFO order
//! assert_eq!(u8pool.pop(), Some(&b"third"[..]));
//! assert_eq!(u8pool.pop(), Some(&b"second"[..]));
//! assert_eq!(u8pool.pop(), Some(&b"first"[..]));
//!
//! assert!(u8pool.is_empty());
//!
//! // Pop returns None when empty
//! assert_eq!(u8pool.pop(), None);
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
//! u8pool.push(b"vector_data").unwrap();
//!
//! // Both interfaces work on the same underlying data
//! assert_eq!(u8pool.get(0).unwrap(), b"stack_data");
//! assert_eq!(u8pool.get(1).unwrap(), b"vector_data");
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
//! u8pool.push(b"hello").unwrap();
//! u8pool.push(b"world").unwrap();
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
