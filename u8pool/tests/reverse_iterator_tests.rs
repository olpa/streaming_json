use u8pool::U8Pool;

#[test]
fn test_reverse_iterator_populated_vector() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"first").unwrap();
    u8pool.push(b"second").unwrap();
    u8pool.push(b"third").unwrap();

    let items: Vec<_> = u8pool.iter_rev().collect();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], b"third");
    assert_eq!(items[1], b"second");
    assert_eq!(items[2], b"first");
}

#[test]
fn test_reverse_iterator_empty_vector() {
    let mut buffer = [0u8; 600];
    let u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    let items: Vec<_> = u8pool.iter_rev().collect();
    assert_eq!(items.len(), 0);
}

#[test]
fn test_reverse_iterator_single_item() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"only").unwrap();

    let items: Vec<_> = u8pool.iter_rev().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0], b"only");
}

#[test]
fn test_reverse_iterator_partial_consumption() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"first").unwrap();
    u8pool.push(b"second").unwrap();
    u8pool.push(b"third").unwrap();
    u8pool.push(b"fourth").unwrap();

    let mut iter = u8pool.iter_rev();
    assert_eq!(iter.next(), Some(&b"fourth"[..]));
    assert_eq!(iter.next(), Some(&b"third"[..]));
    // Don't consume the rest
}

#[test]
fn test_reverse_iterator_size_hint() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"first").unwrap();
    u8pool.push(b"second").unwrap();
    u8pool.push(b"third").unwrap();

    let mut iter = u8pool.iter_rev();
    assert_eq!(iter.size_hint(), (3, Some(3)));
    
    iter.next();
    assert_eq!(iter.size_hint(), (2, Some(2)));
    
    iter.next();
    assert_eq!(iter.size_hint(), (1, Some(1)));
    
    iter.next();
    assert_eq!(iter.size_hint(), (0, Some(0)));
}

#[test]
fn test_reverse_iterator_compare_with_forward() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    let items: &[&[u8]] = &[b"alpha", b"beta", b"gamma", b"delta"];
    for item in items {
        u8pool.push(item).unwrap();
    }

    let forward: Vec<_> = u8pool.iter().collect();
    let mut reverse: Vec<_> = u8pool.iter_rev().collect();
    reverse.reverse(); // Reverse it back to compare

    assert_eq!(forward, reverse);
}