use bufvec::BufVec;

#[test]
fn test_dictionary_helper_methods() {
    let mut buffer = [0u8; 200];
    let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    assert!(bufvec.is_key(0));
    assert!(!bufvec.is_value(0));
    assert!(bufvec.is_key(2));
    assert!(bufvec.is_key(4));

    assert!(bufvec.is_value(1));
    assert!(!bufvec.is_key(1));
    assert!(bufvec.is_value(3));
    assert!(bufvec.is_value(5));
}

#[test]
fn test_key_value_pairing_even_elements() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"key1").unwrap();
    bufvec.add(b"value1").unwrap();
    bufvec.add(b"key2").unwrap();
    bufvec.add(b"value2").unwrap();

    assert_eq!(bufvec.len(), 4);
    assert!(!bufvec.has_unpaired_key());
    assert_eq!(bufvec.pairs_count(), 2);

    assert!(bufvec.is_key(0));
    assert!(bufvec.is_value(1));
    assert!(bufvec.is_key(2));
    assert!(bufvec.is_value(3));

    assert_eq!(bufvec.get(0), b"key1");
    assert_eq!(bufvec.get(1), b"value1");
    assert_eq!(bufvec.get(2), b"key2");
    assert_eq!(bufvec.get(3), b"value2");
}

#[test]
fn test_unpaired_key_handling_odd_elements() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"key1").unwrap();
    bufvec.add(b"value1").unwrap();
    bufvec.add(b"key2").unwrap();

    assert_eq!(bufvec.len(), 3);
    assert!(bufvec.has_unpaired_key());
    assert_eq!(bufvec.pairs_count(), 1);

    assert!(bufvec.is_key(0));
    assert!(bufvec.is_value(1));
    assert!(bufvec.is_key(2));

    assert_eq!(bufvec.get(0), b"key1");
    assert_eq!(bufvec.get(1), b"value1");
    assert_eq!(bufvec.get(2), b"key2");
}

#[test]
fn test_dictionary_iterator_even_elements() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"key1").unwrap();
    bufvec.add(b"value1").unwrap();
    bufvec.add(b"key2").unwrap();
    bufvec.add(b"value2").unwrap();

    let pairs: Vec<_> = bufvec.pairs().collect();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], (&b"key1"[..], Some(&b"value1"[..])));
    assert_eq!(pairs[1], (&b"key2"[..], Some(&b"value2"[..])));
}

#[test]
fn test_dictionary_iterator_odd_elements() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"key1").unwrap();
    bufvec.add(b"value1").unwrap();
    bufvec.add(b"key2").unwrap();

    let pairs: Vec<_> = bufvec.pairs().collect();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], (&b"key1"[..], Some(&b"value1"[..])));
    assert_eq!(pairs[1], (&b"key2"[..], None));
}

#[test]
fn test_dictionary_iterator_empty() {
    let mut buffer = [0u8; 200];
    let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    let pairs: Vec<_> = bufvec.pairs().collect();
    assert_eq!(pairs.len(), 0);
}

#[test]
fn test_dictionary_iterator_single_key() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"lonely_key").unwrap();

    let pairs: Vec<_> = bufvec.pairs().collect();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], (&b"lonely_key"[..], None));
}

#[test]
fn test_mixed_usage_vector_and_dictionary() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    // Use vector operations
    bufvec.add(b"name").unwrap();
    bufvec.add(b"Alice").unwrap();
    bufvec.add(b"age").unwrap();
    bufvec.add(b"30").unwrap();

    // Test vector interface still works
    assert_eq!(bufvec.len(), 4);
    assert_eq!(bufvec.get(0), b"name");
    assert_eq!(bufvec.get(1), b"Alice");

    // Test dictionary interface works
    assert_eq!(bufvec.pairs_count(), 2);
    assert!(!bufvec.has_unpaired_key());

    let pairs: Vec<_> = bufvec.pairs().collect();
    assert_eq!(pairs[0], (&b"name"[..], Some(&b"Alice"[..])));
    assert_eq!(pairs[1], (&b"age"[..], Some(&b"30"[..])));

    // Test that popping works and affects dictionary view
    let popped = bufvec.pop();
    assert_eq!(popped, b"30");
    assert!(bufvec.has_unpaired_key());
    assert_eq!(bufvec.pairs_count(), 1);

    let pairs_after_pop: Vec<_> = bufvec.pairs().collect();
    assert_eq!(pairs_after_pop.len(), 2);
    assert_eq!(pairs_after_pop[0], (&b"name"[..], Some(&b"Alice"[..])));
    assert_eq!(pairs_after_pop[1], (&b"age"[..], None));
}

#[test]
fn test_add_key_on_empty_vector() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    assert!(bufvec.add_key(b"key1").is_ok());
    assert_eq!(bufvec.len(), 1);
    assert_eq!(bufvec.get(0), b"key1");
    assert!(bufvec.has_unpaired_key());
}

#[test]
fn test_add_key_replacing_existing_key() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"key1").unwrap();
    bufvec.add(b"value1").unwrap();
    bufvec.add(b"key2").unwrap();

    assert_eq!(bufvec.len(), 3);
    assert!(bufvec.has_unpaired_key());

    // Replace the last key
    assert!(bufvec.add_key(b"newkey2").is_ok());
    assert_eq!(bufvec.len(), 3);
    assert_eq!(bufvec.get(0), b"key1");
    assert_eq!(bufvec.get(1), b"value1");
    assert_eq!(bufvec.get(2), b"newkey2");
    assert!(bufvec.has_unpaired_key());
}

#[test]
fn test_add_key_after_value_normal_add() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"key1").unwrap();
    bufvec.add(b"value1").unwrap();

    assert_eq!(bufvec.len(), 2);
    assert!(!bufvec.has_unpaired_key());

    // Should add normally after a value
    assert!(bufvec.add_key(b"key2").is_ok());
    assert_eq!(bufvec.len(), 3);
    assert_eq!(bufvec.get(0), b"key1");
    assert_eq!(bufvec.get(1), b"value1");
    assert_eq!(bufvec.get(2), b"key2");
    assert!(bufvec.has_unpaired_key());
}

#[test]
fn test_add_value_replacing_existing_value() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"key1").unwrap();
    bufvec.add(b"value1").unwrap();
    bufvec.add(b"key2").unwrap();
    bufvec.add(b"value2").unwrap();

    assert_eq!(bufvec.len(), 4);
    assert!(!bufvec.has_unpaired_key());

    // Replace the last value
    assert!(bufvec.add_value(b"newvalue2").is_ok());
    assert_eq!(bufvec.len(), 4);
    assert_eq!(bufvec.get(0), b"key1");
    assert_eq!(bufvec.get(1), b"value1");
    assert_eq!(bufvec.get(2), b"key2");
    assert_eq!(bufvec.get(3), b"newvalue2");
    assert!(!bufvec.has_unpaired_key());
}

#[test]
fn test_add_value_after_key_normal_add() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"key1").unwrap();

    assert_eq!(bufvec.len(), 1);
    assert!(bufvec.has_unpaired_key());

    // Should add normally after a key
    assert!(bufvec.add_value(b"value1").is_ok());
    assert_eq!(bufvec.len(), 2);
    assert_eq!(bufvec.get(0), b"key1");
    assert_eq!(bufvec.get(1), b"value1");
    assert!(!bufvec.has_unpaired_key());
}

#[test]
fn test_add_value_on_empty_vector() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    assert!(bufvec.add_value(b"value1").is_ok());
    assert_eq!(bufvec.len(), 1);
    assert_eq!(bufvec.get(0), b"value1");
    assert!(bufvec.has_unpaired_key()); // Single element at index 0 is considered a key
}
