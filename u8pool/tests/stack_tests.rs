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
fn test_get_out_of_bounds() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"test").unwrap();
    assert!(u8pool.get(1).is_none()); // Should return None
}

#[test]
fn test_clear_operation() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"hello").unwrap();
    u8pool.push(b"world").unwrap();

    assert_eq!(u8pool.len(), 2);

    u8pool.clear();

    assert_eq!(u8pool.len(), 0);
    assert!(u8pool.is_empty());
}

#[test]
fn test_pop_operation() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"hello").unwrap();
    u8pool.push(b"world").unwrap();

    let popped = u8pool.pop();
    assert_eq!(popped, Some(&b"world"[..]));
    assert_eq!(u8pool.len(), 1);

    let popped = u8pool.pop();
    assert_eq!(popped, Some(&b"hello"[..]));
    assert_eq!(u8pool.len(), 0);

    assert_eq!(u8pool.pop(), None);
}

#[test]
fn test_custom_max_slices() {
    let mut buffer = [0u8; 100];
    let mut u8pool = U8Pool::new(&mut buffer, 3).unwrap();

    u8pool.push(b"test").unwrap();
    u8pool.push(b"hello").unwrap();
    u8pool.push(b"world").unwrap();

    // Should fail on 4th slice
    assert!(u8pool.push(b"fail").is_err());

    assert_eq!(u8pool.get(0).unwrap(), b"test");
    assert_eq!(u8pool.get(1).unwrap(), b"hello");
    assert_eq!(u8pool.get(2).unwrap(), b"world");
    assert_eq!(u8pool.len(), 3);
}

#[test]
fn test_stack_push_operations() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    assert!(u8pool.is_empty());
    assert_eq!(u8pool.len(), 0);

    // Test push operations
    assert!(u8pool.push(b"first").is_ok());
    assert_eq!(u8pool.len(), 1);
    assert!(!u8pool.is_empty());

    assert!(u8pool.push(b"second").is_ok());
    assert_eq!(u8pool.len(), 2);

    assert!(u8pool.push(b"third").is_ok());
    assert_eq!(u8pool.len(), 3);

    // Verify elements are in correct order (LIFO for stack perspective)
    assert_eq!(u8pool.get(0).unwrap(), b"first");
    assert_eq!(u8pool.get(1).unwrap(), b"second");
    assert_eq!(u8pool.get(2).unwrap(), b"third");
}

#[test]
fn test_stack_push_pop_operations() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Push elements
    u8pool.push(b"first").unwrap();
    u8pool.push(b"second").unwrap();
    u8pool.push(b"third").unwrap();

    assert_eq!(u8pool.len(), 3);

    // Pop elements in LIFO order
    assert_eq!(u8pool.pop(), Some(&b"third"[..]));
    assert_eq!(u8pool.len(), 2);

    assert_eq!(u8pool.pop(), Some(&b"second"[..]));
    assert_eq!(u8pool.len(), 1);

    assert_eq!(u8pool.pop(), Some(&b"first"[..]));
    assert_eq!(u8pool.len(), 0);
    assert!(u8pool.is_empty());

    // Test empty pop returns None
    assert_eq!(u8pool.pop(), None);
}

#[test]
fn test_stack_buffer_overflow() {
    let mut buffer = [0u8; 150]; // 32*4=128 metadata + 22 data
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Fill buffer to near capacity
    u8pool.push(b"data1").unwrap();
    u8pool.push(b"data2").unwrap();

    // Try to push data that won't fit
    let large_data = vec![b'x'; 20]; // Should exceed remaining space
    assert!(u8pool.push(&large_data).is_err());

    // Stack should be unchanged
    assert_eq!(u8pool.len(), 2);
}

#[test]
fn test_stack_utility_methods() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test utility methods on empty stack
    assert!(u8pool.is_empty());
    assert_eq!(u8pool.len(), 0);

    // Add elements and test utilities
    u8pool.push(b"element").unwrap();
    assert!(!u8pool.is_empty());
    assert_eq!(u8pool.len(), 1);

    u8pool.push(b"another").unwrap();
    assert!(!u8pool.is_empty());
    assert_eq!(u8pool.len(), 2);

    // Clear and test utilities
    u8pool.clear();
    assert!(u8pool.is_empty());
    assert_eq!(u8pool.len(), 0);
}
