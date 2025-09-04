use u8pool::U8Pool;

#[test]
fn test_buffer_initialization() {
    let mut buffer = [0u8; 600];
    let u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    assert_eq!(u8pool.len(), 0);
    assert!(u8pool.is_empty());
}

#[test]
fn test_bounds_checking_empty_buffer() {
    let mut buffer = [0u8; 0];
    assert!(U8Pool::with_default_max_slices(&mut buffer).is_err());

    let mut buffer = [0u8; 600];
    let u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    assert!(u8pool.get(0).is_none());
}

#[test]
fn test_pop_empty_vector() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();
    assert_eq!(u8pool.pop(), None); // Should return None
}

#[test]
fn test_memory_layout_integrity() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"hello").unwrap();
    u8pool.add(b"world").unwrap();

    assert_eq!(u8pool.get(0).unwrap(), b"hello");
    assert_eq!(u8pool.get(1).unwrap(), b"world");
    assert_eq!(u8pool.len(), 2);
}

#[test]
fn test_no_internal_allocation() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"test").unwrap();

    // Verify data is stored correctly in the buffer
    assert_eq!(u8pool.get(0).unwrap(), b"test");
    assert_eq!(u8pool.len(), 1);
}

#[test]
fn test_buffer_overflow() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Fill up the buffer with data
    assert!(u8pool.add(b"hello").is_ok());
    assert!(u8pool.add(b"world").is_ok());

    // Try to add more data than fits in the remaining space
    assert!(u8pool
        .add(b"this_is_a_very_long_string_that_should_definitely_not_fit_in_the_remaining_space_because_it_is_way_too_long_and_exceeds_the_buffer_capacity_by_a_lot")
        .is_err());
}

#[test]
fn test_bounds_checking() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"test").unwrap();

    assert_eq!(u8pool.get(0).unwrap(), b"test");
    assert!(u8pool.get(1).is_none());
}

#[test]
fn test_get_out_of_bounds() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"test").unwrap();
    assert!(u8pool.get(1).is_none()); // Should return None
}

#[test]
fn test_clear_operation() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"hello").unwrap();
    u8pool.add(b"world").unwrap();

    assert_eq!(u8pool.len(), 2);

    u8pool.clear();

    assert_eq!(u8pool.len(), 0);
    assert!(u8pool.is_empty());
}

#[test]
fn test_pop_operation() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"hello").unwrap();
    u8pool.add(b"world").unwrap();

    let popped = u8pool.pop();
    assert_eq!(popped, Some(&b"world"[..]));
    assert_eq!(u8pool.len(), 1);

    let popped = u8pool.pop();
    assert_eq!(popped, Some(&b"hello"[..]));
    assert_eq!(u8pool.len(), 0);

    assert!(u8pool.try_pop().is_err());
}

#[test]
fn test_custom_max_slices() {
    let mut buffer = [0u8; 100];
    let mut u8pool = U8Pool::new(&mut buffer, 3).unwrap();

    u8pool.add(b"test").unwrap();
    u8pool.add(b"hello").unwrap();
    u8pool.add(b"world").unwrap();

    // Should fail on 4th slice
    assert!(u8pool.add(b"fail").is_err());

    assert_eq!(u8pool.get(0).unwrap(), b"test");
    assert_eq!(u8pool.get(1).unwrap(), b"hello");
    assert_eq!(u8pool.get(2).unwrap(), b"world");
    assert_eq!(u8pool.len(), 3);
}

