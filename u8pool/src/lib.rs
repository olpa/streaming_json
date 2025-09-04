#![no_std]

//! `U8Pool`: A zero-allocation stack implementation using client-provided buffers.
//!
//! `U8Pool` is primarily a stack data structure with additional vector functions for indexed access.
//! It uses a single client-provided buffer for storage. All operations are bounds-checked and
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

//! # Stack Interface (Primary)
//!
//! `U8Pool` is primarily a stack, providing LIFO (Last In, First Out) operations through `push()` and `pop()`:
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
//! The stack provides additional vector functions for indexed access:
//!
//! ```
//! # use u8pool::U8Pool;
//! let mut buffer = [0u8; 600];
//! let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Push elements using stack interface
//! u8pool.push(b"first_item").unwrap();
//! u8pool.push(b"second_item").unwrap();
//!
//! // Access elements using vector functions
//! assert_eq!(u8pool.get(0).unwrap(), b"first_item");
//! assert_eq!(u8pool.get(1).unwrap(), b"second_item");
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
