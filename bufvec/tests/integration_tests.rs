use bufvec::BufVec;

#[test]
fn test_buffer_initialization() {
    let mut buffer = [0u8; 200];
    let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    assert_eq!(bufvec.len(), 0);
    assert!(bufvec.is_empty());
    assert_eq!(bufvec.buffer_capacity(), 200);
    assert_eq!(bufvec.max_slices(), 8);
    assert_eq!(bufvec.used_bytes(), 128); // metadata section takes 128 bytes (8 slices * 16 bytes)
    assert!(bufvec.available_bytes() > 0);
}

#[test]
fn test_bounds_checking_empty_buffer() {
    let mut buffer = [0u8; 0];
    assert!(BufVec::with_default_max_slices(&mut buffer).is_err());

    let mut buffer = [0u8; 200];
    let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    assert!(bufvec.try_get(0).is_err());
}

#[test]
#[should_panic(expected = "Cannot pop from empty vector")]
fn test_pop_empty_vector() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
    bufvec.pop(); // Should panic
}

#[test]
fn test_memory_layout_integrity() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"hello").unwrap();
    bufvec.add(b"world").unwrap();

    assert_eq!(bufvec.get(0), b"hello");
    assert_eq!(bufvec.get(1), b"world");
    assert_eq!(bufvec.len(), 2);
}

#[test]
fn test_no_internal_allocation() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"test").unwrap();

    // Verify data is stored correctly in the buffer
    assert_eq!(bufvec.get(0), b"test");
    assert_eq!(bufvec.len(), 1);
}

#[test]
fn test_buffer_overflow() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Fill up the buffer with data
    assert!(bufvec.add(b"hello").is_ok());
    assert!(bufvec.add(b"world").is_ok());

    // Try to add more data than fits in the remaining space
    assert!(bufvec
        .add(b"this_is_a_very_long_string_that_should_not_fit_in_the_remaining_space")
        .is_err());
}

#[test]
fn test_bounds_checking() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"test").unwrap();

    assert_eq!(bufvec.get(0), b"test");
    assert!(bufvec.try_get(1).is_err());
}

#[test]
#[should_panic(expected = "Index 1 out of bounds for vector of length 1")]
fn test_get_out_of_bounds() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"test").unwrap();
    let _ = bufvec.get(1); // Should panic
}

#[test]
fn test_clear_operation() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"hello").unwrap();
    bufvec.add(b"world").unwrap();

    assert_eq!(bufvec.len(), 2);

    bufvec.clear();

    assert_eq!(bufvec.len(), 0);
    assert!(bufvec.is_empty());
}

#[test]
fn test_pop_operation() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"hello").unwrap();
    bufvec.add(b"world").unwrap();

    let popped = bufvec.pop();
    assert_eq!(popped, b"world");
    assert_eq!(bufvec.len(), 1);

    let popped = bufvec.pop();
    assert_eq!(popped, b"hello");
    assert_eq!(bufvec.len(), 0);

    assert!(bufvec.try_pop().is_err());
}

#[test]
fn test_custom_max_slices() {
    let mut buffer = [0u8; 100];
    let mut bufvec = BufVec::new(&mut buffer, 3).unwrap();

    bufvec.add(b"test").unwrap();
    bufvec.add(b"hello").unwrap();
    bufvec.add(b"world").unwrap();

    // Should fail on 4th slice
    assert!(bufvec.add(b"fail").is_err());

    assert_eq!(bufvec.get(0), b"test");
    assert_eq!(bufvec.get(1), b"hello");
    assert_eq!(bufvec.get(2), b"world");
    assert_eq!(bufvec.len(), 3);
    assert_eq!(bufvec.max_slices(), 3);
}

#[test]
fn test_fixed_descriptor_functionality() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Test that derived values work correctly
    assert_eq!(bufvec.max_slices(), 8);
    assert_eq!(bufvec.data_used(), 0);

    bufvec.add(b"test").unwrap();
    assert_eq!(bufvec.data_used(), 4);

    bufvec.add(b"hello").unwrap();
    assert_eq!(bufvec.data_used(), 9);

    bufvec.pop();
    assert_eq!(bufvec.data_used(), 4);

    bufvec.clear();
    assert_eq!(bufvec.data_used(), 0);
}
