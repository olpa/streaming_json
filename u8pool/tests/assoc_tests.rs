use u8pool::{U8Pool, U8PoolError};

#[derive(Debug, PartialEq, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn test_push_pop_assoc_basic() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    let key = Point { x: 42, y: 100 };
    let data = b"hello";

    pool.push_assoc(key, data).unwrap();
    assert_eq!(pool.len(), 1);

    let (retrieved_key, retrieved_data) = pool.get_assoc::<Point>(0).unwrap();
    assert_eq!(*retrieved_key, Point { x: 42, y: 100 });
    assert_eq!(retrieved_data, b"hello");

    let (popped_key, popped_data) = pool.pop_assoc::<Point>().unwrap();
    assert_eq!(*popped_key, Point { x: 42, y: 100 });
    assert_eq!(popped_data, b"hello");
    assert_eq!(pool.len(), 0);
}

#[test]
fn test_push_pop_assoc_multiple() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Push multiple points
    pool.push_assoc(Point { x: 10, y: 20 }, b"first").unwrap();
    pool.push_assoc(Point { x: 30, y: 40 }, b"second").unwrap();
    pool.push_assoc(Point { x: 50, y: 60 }, b"third").unwrap();

    assert_eq!(pool.len(), 3);

    let (key1, data1) = pool.get_assoc::<Point>(0).unwrap();
    assert_eq!(*key1, Point { x: 10, y: 20 });
    assert_eq!(data1, b"first");

    let (key2, data2) = pool.get_assoc::<Point>(1).unwrap();
    assert_eq!(*key2, Point { x: 30, y: 40 });
    assert_eq!(data2, b"second");

    let (key3, data3) = pool.get_assoc::<Point>(2).unwrap();
    assert_eq!(*key3, Point { x: 50, y: 60 });
    assert_eq!(data3, b"third");

    // Pop in LIFO order
    let (key3, data3) = pool.pop_assoc::<Point>().unwrap();
    assert_eq!(*key3, Point { x: 50, y: 60 });
    assert_eq!(data3, b"third");
    assert_eq!(pool.len(), 2);

    let (key2, data2) = pool.pop_assoc::<Point>().unwrap();
    assert_eq!(*key2, Point { x: 30, y: 40 });
    assert_eq!(data2, b"second");
    assert_eq!(pool.len(), 1);

    let (key1, data1) = pool.pop_assoc::<Point>().unwrap();
    assert_eq!(*key1, Point { x: 10, y: 20 });
    assert_eq!(data1, b"first");
    assert_eq!(pool.len(), 0);
}

#[test]
fn test_push_assoc_zero_data() {
    let mut buffer = [0u8; 64];
    let mut pool = U8Pool::new(&mut buffer, 4).unwrap();

    let key = Point { x: 123, y: 456 };
    let data = b"";

    pool.push_assoc(key, data).unwrap();
    assert_eq!(pool.len(), 1);

    let (retrieved_key, retrieved_data) = pool.get_assoc::<Point>(0).unwrap();
    assert_eq!(*retrieved_key, Point { x: 123, y: 456 });
    assert_eq!(retrieved_data.len(), 0);
}

#[test]
fn test_push_assoc_buffer_overflow() {
    let mut buffer = [0u8; 64]; // Small buffer
    let mut pool = U8Pool::new(&mut buffer, 2).unwrap();

    let key = Point { x: 42, y: 84 };
    let large_data = [0u8; 100]; // Too large for buffer

    let result = pool.push_assoc(key, &large_data);
    assert!(matches!(result, Err(U8PoolError::BufferOverflow { .. })));
}

#[test]
fn test_pop_assoc_empty() {
    let mut buffer = [0u8; 64];
    let mut pool = U8Pool::new(&mut buffer, 4).unwrap();

    let result = pool.pop_assoc::<Point>();
    assert!(result.is_none());
}

#[test]
fn test_get_assoc_out_of_bounds() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    pool.push_assoc(Point { x: 42, y: 84 }, b"data").unwrap();

    // Valid access
    assert!(pool.get_assoc::<Point>(0).is_some());

    // Out of bounds access
    assert!(pool.get_assoc::<Point>(1).is_none());
    assert!(pool.get_assoc::<Point>(100).is_none());
}

#[test]
fn test_assoc_mixed_with_regular() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Mix regular and associated pushes
    pool.push(b"regular1").unwrap();
    pool.push_assoc(Point { x: 100, y: 200 }, b"assoc1")
        .unwrap();
    pool.push(b"regular2").unwrap();
    pool.push_assoc(Point { x: 300, y: 400 }, b"assoc2")
        .unwrap();

    assert_eq!(pool.len(), 4);

    // Check regular slices
    assert_eq!(pool.get(0).unwrap(), b"regular1");
    assert_eq!(pool.get(2).unwrap(), b"regular2");

    // Check associated slices (accessing as regular should work for data portion)
    let (key1, data1) = pool.get_assoc::<Point>(1).unwrap();
    assert_eq!(*key1, Point { x: 100, y: 200 });
    assert_eq!(data1, b"assoc1");

    let (key2, data2) = pool.get_assoc::<Point>(3).unwrap();
    assert_eq!(*key2, Point { x: 300, y: 400 });
    assert_eq!(data2, b"assoc2");
}

#[test]
fn test_assoc_iterator_forward() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Push some associated data
    pool.push_assoc(Point { x: 10, y: 15 }, b"a").unwrap();
    pool.push_assoc(Point { x: 20, y: 25 }, b"bb").unwrap();
    pool.push_assoc(Point { x: 30, y: 35 }, b"ccc").unwrap();

    let items: Vec<_> = pool.iter_assoc::<Point>().collect();

    assert_eq!(items.len(), 3);
    assert_eq!(*items[0].0, Point { x: 10, y: 15 });
    assert_eq!(items[0].1, b"a");
    assert_eq!(*items[1].0, Point { x: 20, y: 25 });
    assert_eq!(items[1].1, b"bb");
    assert_eq!(*items[2].0, Point { x: 30, y: 35 });
    assert_eq!(items[2].1, b"ccc");
}

#[test]
fn test_assoc_iterator_reverse() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Push some associated data
    pool.push_assoc(Point { x: 10, y: 15 }, b"a").unwrap();
    pool.push_assoc(Point { x: 20, y: 25 }, b"bb").unwrap();
    pool.push_assoc(Point { x: 30, y: 35 }, b"ccc").unwrap();

    let items: Vec<_> = pool.iter_assoc_rev::<Point>().collect();

    assert_eq!(items.len(), 3);
    assert_eq!(*items[0].0, Point { x: 30, y: 35 });
    assert_eq!(items[0].1, b"ccc");
    assert_eq!(*items[1].0, Point { x: 20, y: 25 });
    assert_eq!(items[1].1, b"bb");
    assert_eq!(*items[2].0, Point { x: 10, y: 15 });
    assert_eq!(items[2].1, b"a");
}

#[test]
fn test_assoc_iterator_empty() {
    let mut buffer = [0u8; 64];
    let pool = U8Pool::new(&mut buffer, 4).unwrap();

    let items: Vec<_> = pool.iter_assoc::<Point>().collect();
    assert_eq!(items.len(), 0);

    let items_rev: Vec<_> = pool.iter_assoc_rev::<Point>().collect();
    assert_eq!(items_rev.len(), 0);
}

#[test]
fn test_assoc_size_hint() {
    let mut buffer = [0u8; 256];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    pool.push_assoc(Point { x: 1, y: 2 }, b"a").unwrap();
    pool.push_assoc(Point { x: 3, y: 4 }, b"b").unwrap();

    let iter = pool.iter_assoc::<Point>();
    assert_eq!(iter.size_hint(), (2, Some(2)));

    let iter_rev = pool.iter_assoc_rev::<Point>();
    assert_eq!(iter_rev.size_hint(), (2, Some(2)));
}

#[test]
fn test_assoc_slice_limit_exceeded() {
    let mut buffer = [0u8; 1024];
    let mut pool = U8Pool::new(&mut buffer, 2).unwrap(); // Limit to 2 slices

    pool.push_assoc(Point { x: 1, y: 10 }, b"first").unwrap();
    pool.push_assoc(Point { x: 2, y: 20 }, b"second").unwrap();

    // Third push should fail
    let result = pool.push_assoc(Point { x: 3, y: 30 }, b"third");
    assert!(matches!(
        result,
        Err(U8PoolError::SliceLimitExceeded { max_slices: 2 })
    ));
}
