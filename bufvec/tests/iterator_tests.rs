use bufvec::BufVec;

#[test]
fn test_iterator_empty_vector() {
    let mut buffer = [0u8; 200];
    let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    let mut iter = bufvec.into_iter();
    assert_eq!(iter.next(), None);
    assert_eq!(iter.size_hint(), (0, Some(0)));
}

#[test]
fn test_iterator_populated_vector() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"hello").unwrap();
    bufvec.add(b"world").unwrap();
    bufvec.add(b"test").unwrap();

    let mut iter = bufvec.into_iter();
    assert_eq!(iter.size_hint(), (3, Some(3)));

    assert_eq!(iter.next(), Some(&b"hello"[..]));
    assert_eq!(iter.size_hint(), (2, Some(2)));

    assert_eq!(iter.next(), Some(&b"world"[..]));
    assert_eq!(iter.size_hint(), (1, Some(1)));

    assert_eq!(iter.next(), Some(&b"test"[..]));
    assert_eq!(iter.size_hint(), (0, Some(0)));

    assert_eq!(iter.next(), None);
}

#[test]
fn test_iterator_consumed_completely() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"a").unwrap();
    bufvec.add(b"b").unwrap();

    let collected: Vec<_> = bufvec.into_iter().collect();
    assert_eq!(collected, vec![&b"a"[..], &b"b"[..]]);
}

#[test]
fn test_iterator_partial_iteration() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"first").unwrap();
    bufvec.add(b"second").unwrap();
    bufvec.add(b"third").unwrap();

    let mut iter = bufvec.into_iter();
    assert_eq!(iter.next(), Some(&b"first"[..]));
    assert_eq!(iter.next(), Some(&b"second"[..]));
    // Iterator should still work after partial consumption
    assert_eq!(iter.size_hint(), (1, Some(1)));
    assert_eq!(iter.next(), Some(&b"third"[..]));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_iterator_lifetime_correctness() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"data").unwrap();

    // Test that iterator can be created and used
    {
        let iter = bufvec.into_iter();
        let first = iter.take(1).next().unwrap();
        assert_eq!(first, b"data");
    }

    // BufVec should still be usable after iterator is dropped
    assert_eq!(bufvec.len(), 1);
    assert_eq!(bufvec.get(0), b"data");
}

#[test]
fn test_for_loop_syntax() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"hello").unwrap();
    bufvec.add(b"world").unwrap();

    let mut results = Vec::new();
    for slice in &bufvec {
        results.push(slice);
    }

    assert_eq!(results, vec![&b"hello"[..], &b"world"[..]]);
}

#[test]
fn test_iter_method() {
    let mut buffer = [0u8; 200];
    let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

    bufvec.add(b"hello").unwrap();
    bufvec.add(b"world").unwrap();

    let collected: Vec<_> = bufvec.iter().collect();
    assert_eq!(collected, vec![&b"hello"[..], &b"world"[..]]);
}
