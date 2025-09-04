use u8pool::U8Pool;

#[test]
fn test_key_value_pairing_even_elements() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"key1").unwrap();
    u8pool.push(b"value1").unwrap();
    u8pool.push(b"key2").unwrap();
    u8pool.push(b"value2").unwrap();

    assert_eq!(u8pool.len(), 4);

    assert_eq!(u8pool.get(0).unwrap(), b"key1");
    assert_eq!(u8pool.get(1).unwrap(), b"value1");
    assert_eq!(u8pool.get(2).unwrap(), b"key2");
    assert_eq!(u8pool.get(3).unwrap(), b"value2");
}

#[test]
fn test_unpaired_key_handling_odd_elements() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"key1").unwrap();
    u8pool.push(b"value1").unwrap();
    u8pool.push(b"key2").unwrap();

    assert_eq!(u8pool.len(), 3);

    assert_eq!(u8pool.get(0).unwrap(), b"key1");
    assert_eq!(u8pool.get(1).unwrap(), b"value1");
    assert_eq!(u8pool.get(2).unwrap(), b"key2");
}

#[test]
fn test_dictionary_iterator_even_elements() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"key1").unwrap();
    u8pool.push(b"value1").unwrap();
    u8pool.push(b"key2").unwrap();
    u8pool.push(b"value2").unwrap();

    let pairs: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], (&b"key1"[..], Some(&b"value1"[..])));
    assert_eq!(pairs[1], (&b"key2"[..], Some(&b"value2"[..])));
}

#[test]
fn test_dictionary_iterator_odd_elements() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"key1").unwrap();
    u8pool.push(b"value1").unwrap();
    u8pool.push(b"key2").unwrap();

    let pairs: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], (&b"key1"[..], Some(&b"value1"[..])));
    assert_eq!(pairs[1], (&b"key2"[..], None));
}

#[test]
fn test_dictionary_iterator_empty() {
    let mut buffer = [0u8; 600];
    let u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    let pairs: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs.len(), 0);
}

#[test]
fn test_dictionary_iterator_single_key() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"lonely_key").unwrap();

    let pairs: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], (&b"lonely_key"[..], None));
}

#[test]
fn test_mixed_usage_vector_and_dictionary() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Use vector operations
    u8pool.push(b"name").unwrap();
    u8pool.push(b"Alice").unwrap();
    u8pool.push(b"age").unwrap();
    u8pool.push(b"30").unwrap();

    // Test vector interface still works
    assert_eq!(u8pool.len(), 4);
    assert_eq!(u8pool.get(0).unwrap(), b"name");
    assert_eq!(u8pool.get(1).unwrap(), b"Alice");

    // Test dictionary interface works
    let pairs: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs[0], (&b"name"[..], Some(&b"Alice"[..])));
    assert_eq!(pairs[1], (&b"age"[..], Some(&b"30"[..])));

    // Test that popping works and affects dictionary view
    let popped = u8pool.pop();
    assert_eq!(popped, Some(&b"30"[..]));

    let pairs_after_pop: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs_after_pop.len(), 2);
    assert_eq!(pairs_after_pop[0], (&b"name"[..], Some(&b"Alice"[..])));
    assert_eq!(pairs_after_pop[1], (&b"age"[..], None));
}