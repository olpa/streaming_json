use u8pool::U8Pool;

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
fn test_stack_interface_doesnt_break_vector_operations() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Mix stack and vector operations
    u8pool.push(b"stack1").unwrap();
    u8pool.push(b"vector1").unwrap();
    u8pool.push(b"stack2").unwrap();

    assert_eq!(u8pool.len(), 3);
    assert_eq!(u8pool.get(0).unwrap(), b"stack1");
    assert_eq!(u8pool.get(1).unwrap(), b"vector1");
    assert_eq!(u8pool.get(2).unwrap(), b"stack2");

    // Stack operations still work
    assert_eq!(u8pool.pop(), Some(&b"stack2"[..]));

    // Vector operations still work
    assert_eq!(u8pool.get(0).unwrap(), b"stack1");
    assert_eq!(u8pool.get(1).unwrap(), b"vector1");

    // Iterator still works
    let collected: Vec<_> = u8pool.iter().collect();
    assert_eq!(collected, vec![&b"stack1"[..], &b"vector1"[..]]);
}

#[test]
fn test_stack_buffer_overflow() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Fill buffer to near capacity
    u8pool.push(b"data1").unwrap();
    u8pool.push(b"data2").unwrap();

    // Try to push data that won't fit
    let large_data = vec![b'x'; 100];
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
