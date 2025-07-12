use bufvec::{BufVec, BufVecError};

#[test]
fn test_error_zero_max_slices() {
    let mut buffer = [0u8; 200];
    let result = BufVec::new(&mut buffer, 0);
    assert_eq!(
        result.unwrap_err(),
        BufVecError::InvalidConfiguration {
            parameter: "max_slices",
            value: 0
        }
    );
}

#[test]
fn test_error_zero_size_buffer() {
    let mut buffer = [];
    let result = BufVec::new(&mut buffer, 1);
    assert_eq!(result.unwrap_err(), BufVecError::ZeroSizeBuffer);
}

#[test]
fn test_error_buffer_too_small_for_metadata() {
    let mut buffer = [0u8; 10]; // Too small for even 1 slice (16 bytes needed + 1 for data)
    let result = BufVec::new(&mut buffer, 1);
    assert_eq!(
        result.unwrap_err(),
        BufVecError::BufferTooSmall {
            required: 17, // 16 bytes metadata + 1 byte data minimum
            provided: 10
        }
    );
}

#[test]
fn test_error_detailed_buffer_overflow() {
    let mut buffer = [0u8; 150];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Fill buffer to near capacity
    bufvec.add(b"small").unwrap();

    // Try to add data that won't fit
    let large_data = vec![b'x'; 100];
    let result = bufvec.add(&large_data);
    match result.unwrap_err() {
        BufVecError::BufferOverflow {
            requested,
            available,
        } => {
            assert_eq!(requested, 100);
            assert!(available < 100);
        }
        _ => panic!("Expected BufferOverflow error"),
    }
}

#[test]
fn test_error_detailed_index_out_of_bounds() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"test").unwrap();

    let result = bufvec.try_get(5);
    assert_eq!(
        result.unwrap_err(),
        BufVecError::IndexOutOfBounds {
            index: 5,
            length: 1
        }
    );
}

#[test]
fn test_error_slice_limit_exceeded() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::new(&mut buffer, 2).unwrap(); // Only 2 slices allowed

    bufvec.add(b"first").unwrap();
    bufvec.add(b"second").unwrap();

    let result = bufvec.add(b"third");
    assert_eq!(
        result.unwrap_err(),
        BufVecError::SliceLimitExceeded { max_slices: 2 }
    );
}

#[test]
fn test_error_empty_vector_operations() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Test try_pop on empty vector
    assert_eq!(bufvec.try_pop().unwrap_err(), BufVecError::EmptyVector);

    // Test try_top on empty vector
    assert_eq!(bufvec.try_top().unwrap_err(), BufVecError::EmptyVector);
}

#[test]
fn test_error_messages_quality() {
    let mut buffer = [0u8; 10];
    let error = BufVec::new(&mut buffer, 1).unwrap_err();
    let message = format!("{}", error);
    assert!(message.contains("17 bytes required"));
    assert!(message.contains("10 bytes provided"));

    let mut buffer = [0u8; 200];
    let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
    let error = bufvec.try_get(0).unwrap_err();
    let message = format!("{}", error);
    assert!(message.contains("Index 0 out of bounds"));
    assert!(message.contains("length 0"));
}

#[test]
fn test_error_types_implement_standard_traits() {
    let error = BufVecError::EmptyVector;

    // Test Debug
    let debug_str = format!("{:?}", error);
    assert!(!debug_str.is_empty());

    // Test Display
    let display_str = format!("{}", error);
    assert!(!display_str.is_empty());

    // Test Clone
    let cloned = error.clone();
    assert_eq!(error, cloned);

    // Test PartialEq
    assert_eq!(error, BufVecError::EmptyVector);
    assert_ne!(error, BufVecError::ZeroSizeBuffer);

    // Test Error trait
    let _: &dyn std::error::Error = &error;
}

#[test]
fn test_comprehensive_error_scenarios() {
    // Test all error variants have proper error messages
    let errors = [
        BufVecError::BufferOverflow {
            requested: 100,
            available: 50,
        },
        BufVecError::IndexOutOfBounds {
            index: 5,
            length: 2,
        },
        BufVecError::EmptyVector,
        BufVecError::BufferTooSmall {
            required: 100,
            provided: 50,
        },
        BufVecError::SliceLimitExceeded { max_slices: 8 },
        BufVecError::ZeroSizeBuffer,
        BufVecError::InvalidConfiguration {
            parameter: "test",
            value: 0,
        },
    ];

    for error in &errors {
        let message = format!("{}", error);
        assert!(
            !message.is_empty(),
            "Error message should not be empty for {:?}",
            error
        );
        assert!(
            message.len() > 10,
            "Error message should be descriptive for {:?}",
            error
        );
    }
}