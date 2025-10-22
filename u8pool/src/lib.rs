#![no_std]
#![doc = include_str!("../README.md")]

mod core;
mod error;
mod iter;
mod slice_descriptor;

pub use core::U8Pool;
pub use error::U8PoolError;
pub use iter::{U8PoolAssocIter, U8PoolAssocRevIter, U8PoolIter, U8PoolPairIter, U8PoolRevIter};
