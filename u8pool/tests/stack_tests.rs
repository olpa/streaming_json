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

#[test]
fn test_push_returns_reference_to_stored_value() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test that push returns a reference to the stored data
    let original_data = b"hello world";
    let stored_ref = u8pool.push(original_data).unwrap();

    // The content behind the reference should match the original data
    assert_eq!(stored_ref, original_data);
    assert_eq!(stored_ref.len(), original_data.len());

    // Verify push() and get() return references that point to the same memory location
    // (Different reference objects, but pointing to the same underlying data)
    let stored_ptr = stored_ref.as_ptr();
    let stored_len = stored_ref.len();
    // End the mutable borrow by letting the reference go out of scope
    let _ = stored_ref;

    // Now we can get an immutable reference - this should point to the same memory
    let get_ref = u8pool.get(0).unwrap();
    let get_ptr = get_ref.as_ptr();
    let get_len = get_ref.len();

    assert_eq!(
        stored_ptr, get_ptr,
        "push() and get() should return references pointing to the same memory location"
    );
    assert_eq!(
        stored_len, get_len,
        "push() and get() should return references with the same length"
    );
}

#[test]
fn test_push_returns_independent_references() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test that each push() returns a reference pointing to the correct memory location
    // Each push() reference should point to the same memory as the corresponding get() reference
    let first_ref = u8pool.push(b"first").unwrap();
    assert_eq!(first_ref, b"first"); // Content should match
    let first_ptr = first_ref.as_ptr();
    assert_eq!(
        first_ptr,
        u8pool.get(0).unwrap().as_ptr(),
        "First push and get(0) should point to same memory"
    );

    let second_ref = u8pool.push(b"second").unwrap();
    assert_eq!(second_ref, b"second"); // Content should match
    let second_ptr = second_ref.as_ptr();
    assert_eq!(
        second_ptr,
        u8pool.get(1).unwrap().as_ptr(),
        "Second push and get(1) should point to same memory"
    );

    let third_ref = u8pool.push(b"third").unwrap();
    assert_eq!(third_ref, b"third"); // Content should match
    let third_ptr = third_ref.as_ptr();
    assert_eq!(
        third_ptr,
        u8pool.get(2).unwrap().as_ptr(),
        "Third push and get(2) should point to same memory"
    );

    // Verify each push uses different memory locations (independence)
    assert_ne!(first_ptr, second_ptr);
    assert_ne!(second_ptr, third_ptr);
    assert_ne!(first_ptr, third_ptr);

    // Verify all stored content is accessible and correct
    assert_eq!(u8pool.get(0).unwrap(), b"first");
    assert_eq!(u8pool.get(1).unwrap(), b"second");
    assert_eq!(u8pool.get(2).unwrap(), b"third");
}

#[test]
fn test_push_return_value_with_empty_data() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test pushing empty slice
    let empty_ref = u8pool.push(b"").unwrap();
    assert_eq!(empty_ref, b""); // Content should match
    assert_eq!(empty_ref.len(), 0);

    // Verify push() and get() point to the same memory location even for empty data
    let empty_ptr = empty_ref.as_ptr();
    let empty_len = empty_ref.len();
    let _ = empty_ref;

    let get_empty_ref = u8pool.get(0).unwrap();
    let get_empty_ptr = get_empty_ref.as_ptr();
    let get_empty_len = get_empty_ref.len();

    assert_eq!(
        empty_ptr, get_empty_ptr,
        "push() and get() should point to same memory location for empty data"
    );
    assert_eq!(empty_len, get_empty_len);
}

// Tests for top() method - returns reference to top slice without removing it

#[test]
fn test_top_basic() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    pool.push(b"first").unwrap();
    pool.push(b"second").unwrap();
    pool.push(b"third").unwrap();

    // Top should return the last pushed item
    let top_ref = pool.top().unwrap();
    assert_eq!(top_ref, b"third");
    assert_eq!(pool.len(), 3); // Stack should still have all items

    // Verify other elements are unchanged
    assert_eq!(pool.get(0).unwrap(), b"first");
    assert_eq!(pool.get(1).unwrap(), b"second");
    assert_eq!(pool.get(2).unwrap(), b"third");
}

#[test]
fn test_top_empty_pool() {
    let mut buffer = [0u8; 256];
    let pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    assert!(pool.top().is_none());
}

#[test]
fn test_top_single_item() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    pool.push(b"only").unwrap();
    let top_ref = pool.top().unwrap();
    assert_eq!(top_ref, b"only");
    assert_eq!(pool.len(), 1);
}

#[test]
fn test_top_after_pop() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    pool.push(b"first").unwrap();
    pool.push(b"second").unwrap();
    pool.push(b"third").unwrap();

    pool.pop().unwrap(); // Remove "third"
    let top_ref = pool.top().unwrap();
    assert_eq!(top_ref, b"second"); // Now "second" is top
    assert_eq!(pool.len(), 2);
}

#[test]
fn test_top_pointer_equality_with_get() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    pool.push(b"test data").unwrap();

    // top() and get() should return references pointing to the same memory
    let top_ref = pool.top().unwrap();
    let get_ref = pool.get(0).unwrap();

    assert_eq!(top_ref.as_ptr(), get_ref.as_ptr());
    assert_eq!(top_ref, get_ref);
}
