use u8pool::U8Pool;

#[derive(Debug, PartialEq, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn test_iterator_empty_vector() {
    let mut buffer = [0u8; 600];
    let u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    let mut iter = u8pool.into_iter();
    assert_eq!(iter.next(), None);
    assert_eq!(iter.size_hint(), (0, Some(0)));
}

#[test]
fn test_iterator_populated_vector() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"hello").unwrap();
    u8pool.push(b"world").unwrap();
    u8pool.push(b"test").unwrap();

    let mut iter = u8pool.into_iter();
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
fn test_iterator_single_item() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"only").unwrap();

    let mut iter = u8pool.into_iter();
    assert_eq!(iter.size_hint(), (1, Some(1)));
    assert_eq!(iter.next(), Some(&b"only"[..]));
    assert_eq!(iter.size_hint(), (0, Some(0)));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_iterator_size_hint() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"first").unwrap();
    u8pool.push(b"second").unwrap();
    u8pool.push(b"third").unwrap();

    let mut iter = u8pool.into_iter();
    assert_eq!(iter.size_hint(), (3, Some(3)));

    iter.next();
    assert_eq!(iter.size_hint(), (2, Some(2)));

    iter.next();
    assert_eq!(iter.size_hint(), (1, Some(1)));

    iter.next();
    assert_eq!(iter.size_hint(), (0, Some(0)));
}

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

#[test]
fn test_iterator_clone() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"first").unwrap();
    u8pool.push(b"second").unwrap();
    u8pool.push(b"third").unwrap();

    let mut iter1 = u8pool.iter();
    let iter2 = iter1.clone();

    // Both iterators should start at the same position
    assert_eq!(iter1.size_hint(), iter2.size_hint());
    assert_eq!(iter1.size_hint(), (3, Some(3)));

    // Advance the first iterator
    assert_eq!(iter1.next(), Some(&b"first"[..]));
    assert_eq!(iter1.size_hint(), (2, Some(2)));

    // The cloned iterator should still be at the original position
    let mut iter2_clone = iter2.clone();
    assert_eq!(iter2_clone.size_hint(), (3, Some(3)));
    assert_eq!(iter2_clone.next(), Some(&b"first"[..]));

    // Continue with first iterator
    assert_eq!(iter1.next(), Some(&b"second"[..]));
    assert_eq!(iter1.next(), Some(&b"third"[..]));
    assert_eq!(iter1.next(), None);
}

#[test]
fn test_reverse_iterator_clone() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"first").unwrap();
    u8pool.push(b"second").unwrap();
    u8pool.push(b"third").unwrap();

    let mut iter1 = u8pool.iter_rev();
    let iter2 = iter1.clone();

    // Both iterators should start at the same position
    assert_eq!(iter1.size_hint(), iter2.size_hint());
    assert_eq!(iter1.size_hint(), (3, Some(3)));

    // Advance the first iterator
    assert_eq!(iter1.next(), Some(&b"third"[..]));
    assert_eq!(iter1.size_hint(), (2, Some(2)));

    // The cloned iterator should still be at the original position
    let mut iter2_clone = iter2.clone();
    assert_eq!(iter2_clone.size_hint(), (3, Some(3)));
    assert_eq!(iter2_clone.next(), Some(&b"third"[..]));

    // Continue with first iterator
    assert_eq!(iter1.next(), Some(&b"second"[..]));
    assert_eq!(iter1.next(), Some(&b"first"[..]));
    assert_eq!(iter1.next(), None);
}

#[test]
fn test_pair_iterator_clone() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push(b"key1").unwrap();
    u8pool.push(b"value1").unwrap();
    u8pool.push(b"key2").unwrap();
    u8pool.push(b"value2").unwrap();

    let mut iter1 = u8pool.pairs();
    let iter2 = iter1.clone();

    // Both iterators should start at the same position
    assert_eq!(iter1.size_hint(), iter2.size_hint());
    assert_eq!(iter1.size_hint(), (2, Some(2)));

    // Advance the first iterator
    assert_eq!(iter1.next(), Some((&b"key1"[..], &b"value1"[..])));
    assert_eq!(iter1.size_hint(), (1, Some(1)));

    // The cloned iterator should still be at the original position
    let mut iter2_clone = iter2.clone();
    assert_eq!(iter2_clone.size_hint(), (2, Some(2)));
    assert_eq!(iter2_clone.next(), Some((&b"key1"[..], &b"value1"[..])));

    // Continue with first iterator
    assert_eq!(iter1.next(), Some((&b"key2"[..], &b"value2"[..])));
    assert_eq!(iter1.next(), None);
}

#[test]
fn test_assoc_iterator_clone() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push_assoc(Point { x: 10, y: 20 }, b"first").unwrap();
    u8pool
        .push_assoc(Point { x: 30, y: 40 }, b"second")
        .unwrap();

    let mut iter1 = unsafe { u8pool.iter_assoc::<Point>() };
    let iter2 = iter1.clone();

    // Both iterators should start at the same position
    assert_eq!(iter1.size_hint(), iter2.size_hint());
    assert_eq!(iter1.size_hint(), (2, Some(2)));

    // Advance the first iterator
    let (val, data) = iter1.next().unwrap();
    assert_eq!(*val, Point { x: 10, y: 20 });
    assert_eq!(data, b"first");
    assert_eq!(iter1.size_hint(), (1, Some(1)));

    // The cloned iterator should still be at the original position
    let mut iter2_clone = iter2.clone();
    assert_eq!(iter2_clone.size_hint(), (2, Some(2)));
    let (val, data) = iter2_clone.next().unwrap();
    assert_eq!(*val, Point { x: 10, y: 20 });
    assert_eq!(data, b"first");

    // Continue with first iterator
    let (val, data) = iter1.next().unwrap();
    assert_eq!(*val, Point { x: 30, y: 40 });
    assert_eq!(data, b"second");
    assert_eq!(iter1.next(), None);
}

#[test]
fn test_assoc_reverse_iterator_clone() {
    let mut buffer = [0u8; 600];
    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    u8pool.push_assoc(Point { x: 10, y: 20 }, b"first").unwrap();
    u8pool
        .push_assoc(Point { x: 30, y: 40 }, b"second")
        .unwrap();

    let mut iter1 = unsafe { u8pool.iter_assoc_rev::<Point>() };
    let iter2 = iter1.clone();

    // Both iterators should start at the same position
    assert_eq!(iter1.size_hint(), iter2.size_hint());
    assert_eq!(iter1.size_hint(), (2, Some(2)));

    // Advance the first iterator
    let (val, data) = iter1.next().unwrap();
    assert_eq!(*val, Point { x: 30, y: 40 });
    assert_eq!(data, b"second");
    assert_eq!(iter1.size_hint(), (1, Some(1)));

    // The cloned iterator should still be at the original position
    let mut iter2_clone = iter2.clone();
    assert_eq!(iter2_clone.size_hint(), (2, Some(2)));
    let (val, data) = iter2_clone.next().unwrap();
    assert_eq!(*val, Point { x: 30, y: 40 });
    assert_eq!(data, b"second");

    // Continue with first iterator
    let (val, data) = iter1.next().unwrap();
    assert_eq!(*val, Point { x: 10, y: 20 });
    assert_eq!(data, b"first");
    assert_eq!(iter1.next(), None);
}
