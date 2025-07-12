/// Error types for BufVec operations
///
/// This module provides error types that are compatible with `no_std` environments.
/// Error types implement `Debug`, `PartialEq`, `Eq`, and `Clone` but do not implement
/// `std::fmt::Display` or `std::error::Error` to maintain `no_std` compatibility.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BufVecError {
    /// Buffer has insufficient space for the requested operation
    BufferOverflow {
        /// Number of bytes requested
        requested: usize,
        /// Number of bytes available
        available: usize,
    },
    /// Index is beyond the current vector length
    IndexOutOfBounds {
        /// Index that was accessed
        index: usize,
        /// Current length of the vector
        length: usize,
    },
    /// Operation attempted on an empty vector
    EmptyVector,
    /// Buffer is too small to hold the required metadata
    BufferTooSmall {
        /// Minimum buffer size required
        required: usize,
        /// Actual buffer size provided
        provided: usize,
    },
    /// Maximum number of slices has been reached
    SliceLimitExceeded {
        /// Maximum number of slices allowed
        max_slices: usize,
    },
    /// Zero-size buffer provided where data storage is required
    ZeroSizeBuffer,
    /// Invalid configuration parameter
    InvalidConfiguration {
        /// Description of the invalid parameter
        parameter: &'static str,
        /// Provided value
        value: usize,
    },
}
