use thiserror::Error;

/// Error types for U8Pool operations
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
    /// Operation attempted on an empty vector
    #[error("Operation attempted on an empty vector")]
    EmptyVector,
    /// Buffer is too small to hold the required metadata
    #[error("Buffer too small: required {required} bytes, but only {provided} bytes provided")]
    BufferTooSmall {
        /// Minimum buffer size required
        required: usize,
        /// Actual buffer size provided
        provided: usize,
    },
    /// Maximum number of slices has been reached
    #[error("Slice limit exceeded: maximum {max_slices} slices allowed")]
    SliceLimitExceeded {
        /// Maximum number of slices allowed
        max_slices: usize,
    },
    /// Zero-size buffer provided where data storage is required
    #[error("Zero-size buffer provided where data storage is required")]
    ZeroSizeBuffer,
    /// Invalid configuration parameter
    #[error("Invalid configuration: parameter '{parameter}' has invalid value {value}")]
    InvalidConfiguration {
        /// Description of the invalid parameter
        parameter: &'static str,
        /// Provided value
        value: usize,
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
