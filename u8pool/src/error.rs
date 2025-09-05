use thiserror::Error;

/// Error types for `U8Pool` operations
#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum U8PoolError {
    /// Buffer has insufficient space for the requested operation
    #[error("Buffer overflow: requested {requested} bytes, but only {available} bytes available")]
    BufferOverflow {
        /// Number of bytes requested
        requested: usize,
        /// Number of bytes available
        available: usize,
    },
    /// Index is beyond the current vector length
    #[error("Index out of bounds: index {index} is beyond vector length {length}")]
    IndexOutOfBounds {
        /// Index that was accessed
        index: usize,
        /// Current length of the vector
        length: usize,
    },
    /// Invalid parameters or buffer provided to `U8Pool::new`
    #[error("Invalid U8Pool initialization: {reason}")]
    InvalidInitialization {
        /// Description of why initialization failed
        reason: &'static str,
    },
    /// Maximum number of slices has been reached
    #[error("Slice limit exceeded: maximum {max_slices} slices allowed")]
    SliceLimitExceeded {
        /// Maximum number of slices allowed
        max_slices: usize,
    },
    /// Value too large for 2-byte storage
    #[error("Value too large: {value} exceeds maximum of {max}")]
    ValueTooLarge {
        /// Value that was too large
        value: usize,
        /// Maximum allowed value
        max: usize,
    },
}
