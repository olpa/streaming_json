use u8pool::U8Pool;

#[test]
fn test_dictionary_helper_methods() {
    let mut buffer = [0u8; 600];
    let u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    assert!(u8pool.is_key(0));
    assert!(!u8pool.is_value(0));
    assert!(u8pool.is_key(2));
    assert!(u8pool.is_key(4));

    assert!(u8pool.is_value(1));
    assert!(!u8pool.is_key(1));
    assert!(u8pool.is_value(3));
    assert!(u8pool.is_value(5));
}

#[test]
fn test_key_value_pairing_even_elements() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"key1").unwrap();
    u8pool.add(b"value1").unwrap();
    u8pool.add(b"key2").unwrap();
    u8pool.add(b"value2").unwrap();

    assert_eq!(u8pool.len(), 4);
    assert!(!u8pool.has_unpaired_key());
    assert_eq!(u8pool.pairs_count(), 2);

    assert!(u8pool.is_key(0));
    assert!(u8pool.is_value(1));
    assert!(u8pool.is_key(2));
    assert!(u8pool.is_value(3));

    assert_eq!(u8pool.get(0).unwrap(), b"key1");
    assert_eq!(u8pool.get(1).unwrap(), b"value1");
    assert_eq!(u8pool.get(2).unwrap(), b"key2");
    assert_eq!(u8pool.get(3).unwrap(), b"value2");
}

#[test]
fn test_unpaired_key_handling_odd_elements() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"key1").unwrap();
    u8pool.add(b"value1").unwrap();
    u8pool.add(b"key2").unwrap();

    assert_eq!(u8pool.len(), 3);
    assert!(u8pool.has_unpaired_key());
    assert_eq!(u8pool.pairs_count(), 1);

    assert!(u8pool.is_key(0));
    assert!(u8pool.is_value(1));
    assert!(u8pool.is_key(2));

    assert_eq!(u8pool.get(0).unwrap(), b"key1");
    assert_eq!(u8pool.get(1).unwrap(), b"value1");
    assert_eq!(u8pool.get(2).unwrap(), b"key2");
}

#[test]
fn test_dictionary_iterator_even_elements() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"key1").unwrap();
    u8pool.add(b"value1").unwrap();
    u8pool.add(b"key2").unwrap();
    u8pool.add(b"value2").unwrap();

    let pairs: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], (&b"key1"[..], Some(&b"value1"[..])));
    assert_eq!(pairs[1], (&b"key2"[..], Some(&b"value2"[..])));
}

#[test]
fn test_dictionary_iterator_odd_elements() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"key1").unwrap();
    u8pool.add(b"value1").unwrap();
    u8pool.add(b"key2").unwrap();

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

    u8pool.add(b"lonely_key").unwrap();

    let pairs: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], (&b"lonely_key"[..], None));
}

#[test]
fn test_mixed_usage_vector_and_dictionary() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Use vector operations
    u8pool.add(b"name").unwrap();
    u8pool.add(b"Alice").unwrap();
    u8pool.add(b"age").unwrap();
    u8pool.add(b"30").unwrap();

    // Test vector interface still works
    assert_eq!(u8pool.len(), 4);
    assert_eq!(u8pool.get(0).unwrap(), b"name");
    assert_eq!(u8pool.get(1).unwrap(), b"Alice");

    // Test dictionary interface works
    assert_eq!(u8pool.pairs_count(), 2);
    assert!(!u8pool.has_unpaired_key());

    let pairs: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs[0], (&b"name"[..], Some(&b"Alice"[..])));
    assert_eq!(pairs[1], (&b"age"[..], Some(&b"30"[..])));

    // Test that popping works and affects dictionary view
    let popped = u8pool.pop();
    assert_eq!(popped, Some(&b"30"[..]));
    assert!(u8pool.has_unpaired_key());
    assert_eq!(u8pool.pairs_count(), 1);

    let pairs_after_pop: Vec<_> = u8pool.pairs().collect();
    assert_eq!(pairs_after_pop.len(), 2);
    assert_eq!(pairs_after_pop[0], (&b"name"[..], Some(&b"Alice"[..])));
    assert_eq!(pairs_after_pop[1], (&b"age"[..], None));
}

#[test]
fn test_add_key_on_empty_vector() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    assert!(u8pool.add_key(b"key1").is_ok());
    assert_eq!(u8pool.len(), 1);
    assert_eq!(u8pool.get(0).unwrap(), b"key1");
    assert!(u8pool.has_unpaired_key());
}

#[test]
fn test_add_key_replacing_existing_key() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"key1").unwrap();
    u8pool.add(b"value1").unwrap();
    u8pool.add(b"key2").unwrap();

    assert_eq!(u8pool.len(), 3);
    assert!(u8pool.has_unpaired_key());

    // Replace the last key
    assert!(u8pool.add_key(b"newkey2").is_ok());
    assert_eq!(u8pool.len(), 3);
    assert_eq!(u8pool.get(0).unwrap(), b"key1");
    assert_eq!(u8pool.get(1).unwrap(), b"value1");
    assert_eq!(u8pool.get(2).unwrap(), b"newkey2");
    assert!(u8pool.has_unpaired_key());
}

#[test]
fn test_add_key_after_value_normal_add() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"key1").unwrap();
    u8pool.add(b"value1").unwrap();

    assert_eq!(u8pool.len(), 2);
    assert!(!u8pool.has_unpaired_key());

    // Should add normally after a value
    assert!(u8pool.add_key(b"key2").is_ok());
    assert_eq!(u8pool.len(), 3);
    assert_eq!(u8pool.get(0).unwrap(), b"key1");
    assert_eq!(u8pool.get(1).unwrap(), b"value1");
    assert_eq!(u8pool.get(2).unwrap(), b"key2");
    assert!(u8pool.has_unpaired_key());
}

#[test]
fn test_add_value_replacing_existing_value() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"key1").unwrap();
    u8pool.add(b"value1").unwrap();
    u8pool.add(b"key2").unwrap();
    u8pool.add(b"value2").unwrap();

    assert_eq!(u8pool.len(), 4);
    assert!(!u8pool.has_unpaired_key());

    // Replace the last value
    assert!(u8pool.add_value(b"newvalue2").is_ok());
    assert_eq!(u8pool.len(), 4);
    assert_eq!(u8pool.get(0).unwrap(), b"key1");
    assert_eq!(u8pool.get(1).unwrap(), b"value1");
    assert_eq!(u8pool.get(2).unwrap(), b"key2");
    assert_eq!(u8pool.get(3).unwrap(), b"newvalue2");
    assert!(!u8pool.has_unpaired_key());
}

#[test]
fn test_add_value_after_key_normal_add() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.add(b"key1").unwrap();

    assert_eq!(u8pool.len(), 1);
    assert!(u8pool.has_unpaired_key());

    // Should add normally after a key
    assert!(u8pool.add_value(b"value1").is_ok());
    assert_eq!(u8pool.len(), 2);
    assert_eq!(u8pool.get(0).unwrap(), b"key1");
    assert_eq!(u8pool.get(1).unwrap(), b"value1");
    assert!(!u8pool.has_unpaired_key());
}

#[test]
fn test_add_value_on_empty_vector() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    assert!(u8pool.add_value(b"value1").is_ok());
    assert_eq!(u8pool.len(), 1);
    assert_eq!(u8pool.get(0).unwrap(), b"value1");
    assert!(u8pool.has_unpaired_key()); // Single element at index 0 is considered a key
}
