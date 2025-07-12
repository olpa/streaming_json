use std::fmt;

/// Error types for BufVec operations
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

impl fmt::Display for BufVecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufVecError::BufferOverflow {
                requested,
                available,
            } => {
                write!(
                    f,
                    "Buffer overflow: requested {requested} bytes, only {available} available"
                )
            }
            BufVecError::IndexOutOfBounds { index, length } => {
                write!(
                    f,
                    "Index {index} out of bounds for vector of length {length}"
                )
            }
            BufVecError::EmptyVector => {
                write!(f, "Operation attempted on empty vector")
            }
            BufVecError::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "Buffer too small: {required} bytes required, {provided} bytes provided"
                )
            }
            BufVecError::SliceLimitExceeded { max_slices } => {
                write!(f, "Maximum number of slices ({max_slices}) exceeded")
            }
            BufVecError::ZeroSizeBuffer => {
                write!(
                    f,
                    "Zero-size buffer provided where data storage is required"
                )
            }
            BufVecError::InvalidConfiguration { parameter, value } => {
                write!(f, "Invalid configuration: {parameter} = {value}")
            }
        }
    }
}

impl std::error::Error for BufVecError {}