use bufvec::BufVec;

#[test]
fn test_stack_push_operations() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    assert!(bufvec.is_empty());
    assert_eq!(bufvec.len(), 0);

    // Test push operations
    assert!(bufvec.push(b"first").is_ok());
    assert_eq!(bufvec.len(), 1);
    assert!(!bufvec.is_empty());

    assert!(bufvec.push(b"second").is_ok());
    assert_eq!(bufvec.len(), 2);

    assert!(bufvec.push(b"third").is_ok());
    assert_eq!(bufvec.len(), 3);

    // Verify elements are in correct order (LIFO for stack perspective)
    assert_eq!(bufvec.get(0), b"first");
    assert_eq!(bufvec.get(1), b"second");
    assert_eq!(bufvec.get(2), b"third");
}

#[test]
fn test_stack_top_operations() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Test try_top on empty stack
    assert!(bufvec.try_top().is_err());

    // Add elements and test top
    bufvec.push(b"bottom").unwrap();
    assert_eq!(bufvec.top(), b"bottom");
    assert_eq!(bufvec.try_top().unwrap(), b"bottom");

    bufvec.push(b"middle").unwrap();
    assert_eq!(bufvec.top(), b"middle");
    assert_eq!(bufvec.try_top().unwrap(), b"middle");

    bufvec.push(b"top").unwrap();
    assert_eq!(bufvec.top(), b"top");
    assert_eq!(bufvec.try_top().unwrap(), b"top");

    // Verify top doesn't modify the stack
    assert_eq!(bufvec.len(), 3);
    assert_eq!(bufvec.top(), b"top");
}

#[test]
#[should_panic(expected = "Cannot peek at top of empty stack")]
fn test_stack_top_empty_panic() {
    let mut buffer = [0u8; 200];
    let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
    let _ = bufvec.top();
}

#[test]
fn test_stack_push_pop_operations() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Push elements
    bufvec.push(b"first").unwrap();
    bufvec.push(b"second").unwrap();
    bufvec.push(b"third").unwrap();

    assert_eq!(bufvec.len(), 3);

    // Pop elements in LIFO order
    assert_eq!(bufvec.pop(), b"third");
    assert_eq!(bufvec.len(), 2);
    assert_eq!(bufvec.top(), b"second");

    assert_eq!(bufvec.pop(), b"second");
    assert_eq!(bufvec.len(), 1);
    assert_eq!(bufvec.top(), b"first");

    assert_eq!(bufvec.pop(), b"first");
    assert_eq!(bufvec.len(), 0);
    assert!(bufvec.is_empty());

    // Test error handling
    assert!(bufvec.try_pop().is_err());
    assert!(bufvec.try_top().is_err());
}

#[test]
fn test_stack_interface_doesnt_break_vector_operations() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Mix stack and vector operations
    bufvec.push(b"stack1").unwrap();
    bufvec.add(b"vector1").unwrap();
    bufvec.push(b"stack2").unwrap();

    assert_eq!(bufvec.len(), 3);
    assert_eq!(bufvec.get(0), b"stack1");
    assert_eq!(bufvec.get(1), b"vector1");
    assert_eq!(bufvec.get(2), b"stack2");

    // Stack operations still work
    assert_eq!(bufvec.top(), b"stack2");
    assert_eq!(bufvec.pop(), b"stack2");

    // Vector operations still work
    assert_eq!(bufvec.get(0), b"stack1");
    assert_eq!(bufvec.get(1), b"vector1");

    // Iterator still works
    let collected: Vec<_> = bufvec.iter().collect();
    assert_eq!(collected, vec![&b"stack1"[..], &b"vector1"[..]]);
}

#[test]
fn test_stack_buffer_overflow() {
    let mut buffer = [0u8; 150];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Fill buffer to near capacity
    bufvec.push(b"data1").unwrap();
    bufvec.push(b"data2").unwrap();

    // Try to push data that won't fit
    let large_data = vec![b'x'; 100];
    assert!(bufvec.push(&large_data).is_err());

    // Stack should be unchanged
    assert_eq!(bufvec.len(), 2);
    assert_eq!(bufvec.top(), b"data2");
}

#[test]
fn test_stack_utility_methods() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Test utility methods on empty stack
    assert!(bufvec.is_empty());
    assert_eq!(bufvec.len(), 0);

    // Add elements and test utilities
    bufvec.push(b"element").unwrap();
    assert!(!bufvec.is_empty());
    assert_eq!(bufvec.len(), 1);

    bufvec.push(b"another").unwrap();
    assert!(!bufvec.is_empty());
    assert_eq!(bufvec.len(), 2);

    // Clear and test utilities
    bufvec.clear();
    assert!(bufvec.is_empty());
    assert_eq!(bufvec.len(), 0);
}