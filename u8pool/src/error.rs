/// Error types for `U8Pool` operations
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum U8PoolError {
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
    /// Invalid parameters or buffer provided to `U8Pool::new`
    InvalidInitialization {
        /// Description of why initialization failed
        reason: &'static str,
    },
    /// Maximum number of slices has been reached
    SliceLimitExceeded {
        /// Maximum number of slices allowed
        max_slices: usize,
    },
    /// Value too large for 2-byte storage
    ValueTooLarge {
        /// Value that was too large
        value: usize,
        /// Maximum allowed value
        max: usize,
    },
}

#[cfg(any(feature = "std", feature = "display"))]
impl core::fmt::Display for U8PoolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            U8PoolError::BufferOverflow {
                requested,
                available,
            } => write!(
                f,
                "Buffer overflow: requested {} bytes, but only {} bytes available",
                requested, available
            ),
            U8PoolError::IndexOutOfBounds { index, length } => write!(
                f,
                "Index out of bounds: index {} is beyond vector length {}",
                index, length
            ),
            U8PoolError::InvalidInitialization { reason } => {
                write!(f, "Invalid U8Pool initialization: {}", reason)
            }
            U8PoolError::SliceLimitExceeded { max_slices } => {
                write!(
                    f,
                    "Slice limit exceeded: maximum {} slices allowed",
                    max_slices
                )
            }
            U8PoolError::ValueTooLarge { value, max } => {
                write!(f, "Value too large: {} exceeds maximum of {}", value, max)
            }
        }
    }
}
