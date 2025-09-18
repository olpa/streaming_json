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

#[test]
fn test_alignment_padding() {
    let mut buffer = [0xAA; 1024]; // Initialize with non-zero pattern to verify padding bytes
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Define test types with different alignment requirements
    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct SingleByte {
        value: u8,
    }

    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct TwoBytes {
        value: u16,
    }

    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct FourBytes {
        value: u32,
    }

    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct EightBytes {
        value: u64,
    }

    #[repr(C)]
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct ComplexStruct {
        small: u8,   // 1 byte
        big: u64,    // 8 bytes, needs 8-byte alignment
        medium: u32, // 4 bytes
    }

    // Add a single byte to create misalignment for subsequent types
    pool.push(b"x").unwrap();

    // Push all test structures
    pool.push_assoc(SingleByte { value: 0x42 }, b"single")
        .unwrap();
    pool.push_assoc(TwoBytes { value: 0x1234 }, b"two").unwrap();
    pool.push_assoc(FourBytes { value: 0x12345678 }, b"four")
        .unwrap();
    pool.push_assoc(
        EightBytes {
            value: 0x123456789ABCDEF0,
        },
        b"eight",
    )
    .unwrap();
    pool.push_assoc(SingleByte { value: 0xFF }, b"y").unwrap(); // Add misalignment before ComplexStruct
    pool.push_assoc(
        ComplexStruct {
            small: 0x12,
            big: 0xFEDCBA9876543210,
            medium: 0x87654321,
        },
        b"complex",
    )
    .unwrap();

    // Pop all and verify correctness
    let (complex_val, complex_data) = pool.pop_assoc::<ComplexStruct>().unwrap();
    assert_eq!(complex_val.small, 0x12);
    assert_eq!(complex_val.big, 0xFEDCBA9876543210);
    assert_eq!(complex_val.medium, 0x87654321);
    assert_eq!(complex_data, b"complex");

    let (single_y_val, single_y_data) = pool.pop_assoc::<SingleByte>().unwrap();
    assert_eq!(single_y_val.value, 0xFF);
    assert_eq!(single_y_data, b"y");

    let (eight_val, eight_data) = pool.pop_assoc::<EightBytes>().unwrap();
    assert_eq!(eight_val.value, 0x123456789ABCDEF0);
    assert_eq!(eight_data, b"eight");

    let (four_val, four_data) = pool.pop_assoc::<FourBytes>().unwrap();
    assert_eq!(four_val.value, 0x12345678);
    assert_eq!(four_data, b"four");

    let (two_val, two_data) = pool.pop_assoc::<TwoBytes>().unwrap();
    assert_eq!(two_val.value, 0x1234);
    assert_eq!(two_data, b"two");

    let (single_val, single_data) = pool.pop_assoc::<SingleByte>().unwrap();
    assert_eq!(single_val.value, 0x42);
    assert_eq!(single_data, b"single");

    let x_data = pool.pop().unwrap();
    assert_eq!(x_data, b"x");

    // Now check the descriptor block
    // Drop the pool to release the borrow on buffer
    drop(pool);

    // The descriptor block is at the start of buffer
    // Each descriptor is 4 bytes: 2 bytes start + 2 bytes length
    // We had 7 slices total: "x", SingleByte+data, TwoBytes+data, FourBytes+data, EightBytes+data, SingleByte("y")+data, ComplexStruct+data

    // Extract descriptor for slice 2 (TwoBytes) - should have 1 byte padding
    let desc2_start = u16::from_le_bytes([buffer[8], buffer[9]]) as usize;

    // Extract descriptor for slice 3 (FourBytes) - should have 3 bytes padding
    let desc3_start = u16::from_le_bytes([buffer[12], buffer[13]]) as usize;

    // Extract descriptor for slice 4 (EightBytes) - should have 7 bytes padding
    let desc4_start = u16::from_le_bytes([buffer[16], buffer[17]]) as usize;

    // Extract descriptor for slice 6 (ComplexStruct) - should have padding due to misalignment from SingleByte("y")
    let desc6_start = u16::from_le_bytes([buffer[24], buffer[25]]) as usize;

    // Data starts after the descriptor block (7 slices * 4 bytes = 28 bytes)
    let data_offset = 28;

    // Check if the stored start positions are actually aligned (they should be)
    // TwoBytes should be 2-byte aligned
    if desc2_start % 2 != 0 {
        panic!(
            "TwoBytes desc2_start {} is not 2-byte aligned!",
            desc2_start
        );
    }

    // FourBytes should be 4-byte aligned
    if desc3_start % 4 != 0 {
        panic!(
            "FourBytes desc3_start {} is not 4-byte aligned!",
            desc3_start
        );
    }

    // EightBytes should be 8-byte aligned
    if desc4_start % 8 != 0 {
        panic!(
            "EightBytes desc4_start {} is not 8-byte aligned!",
            desc4_start
        );
    }

    // ComplexStruct should be 8-byte aligned (due to u64 field)
    if desc6_start % 8 != 0 {
        panic!(
            "ComplexStruct desc6_start {} is not 8-byte aligned!",
            desc6_start
        );
    }

    // Check padding bytes in the actual data buffer
    // Since descriptors should store aligned starts, padding bytes should be BEFORE each slice
    // Based on our test setup, we know the expected padding amounts:

    // TwoBytes: expects 1 byte padding (after "x" + SingleByte we're at odd position, need even)
    assert_eq!(
        buffer[data_offset + desc2_start - 1],
        0xAA,
        "1 padding byte before TwoBytes should be untouched"
    );

    // FourBytes: expects 3 bytes padding (to align to 4-byte boundary)
    assert_eq!(
        buffer[data_offset + desc3_start - 3],
        0xAA,
        "Padding byte 1 before FourBytes should be untouched"
    );
    assert_eq!(
        buffer[data_offset + desc3_start - 2],
        0xAA,
        "Padding byte 2 before FourBytes should be untouched"
    );
    assert_eq!(
        buffer[data_offset + desc3_start - 1],
        0xAA,
        "Padding byte 3 before FourBytes should be untouched"
    );

    // EightBytes: expects 7 bytes padding (to align to 8-byte boundary)
    for i in 1..=7 {
        assert_eq!(
            buffer[data_offset + desc4_start - i],
            0xAA,
            "Padding byte {} before EightBytes should be untouched",
            i
        );
    }

    // ComplexStruct: expects 7 bytes padding (after SingleByte("y"), need 8-byte alignment)
    for i in 1..=7 {
        assert_eq!(
            buffer[data_offset + desc6_start - i],
            0xAA,
            "Padding byte {} before ComplexStruct should be untouched",
            i
        );
    }
}

#[test]
fn test_push_assoc_returns_references_to_stored_values() {
    let mut buffer = [0u8; 600];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test that push_assoc returns references to the stored values
    let original_point = Point { x: 42, y: 100 };
    let original_data = b"hello world";
    let (stored_point_ref, stored_data_ref) =
        pool.push_assoc(original_point, original_data).unwrap();

    // The content behind the references should match the original values
    assert_eq!(*stored_point_ref, original_point);
    assert_eq!(stored_data_ref, original_data);
    assert_eq!(stored_data_ref.len(), original_data.len());

    // Verify push_assoc() and get_assoc() return references pointing to the same memory locations
    // (Different reference objects, but pointing to the same underlying stored data)
    let stored_point_ptr = stored_point_ref as *const Point;
    let stored_data_ptr = stored_data_ref.as_ptr();
    let stored_data_len = stored_data_ref.len();
    let _ = (stored_point_ref, stored_data_ref);

    let (get_point_ref, get_data_ref) = pool.get_assoc::<Point>(0).unwrap();
    let get_point_ptr = get_point_ref as *const Point;
    let get_data_ptr = get_data_ref.as_ptr();
    let get_data_len = get_data_ref.len();

    assert_eq!(
        stored_point_ptr, get_point_ptr,
        "push_assoc() and get_assoc() should return Point references pointing to same memory"
    );
    assert_eq!(
        stored_data_ptr, get_data_ptr,
        "push_assoc() and get_assoc() should return data references pointing to same memory"
    );
    assert_eq!(
        stored_data_len, get_data_len,
        "push_assoc() and get_assoc() should return references with same data length"
    );
}

#[test]
fn test_push_assoc_returns_independent_references() {
    let mut buffer = [0u8; 600];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test that each push_assoc() returns references pointing to the correct memory locations
    // Each push_assoc() reference should point to the same memory as the corresponding get_assoc() reference
    let (first_point, first_data) = pool.push_assoc(Point { x: 10, y: 20 }, b"first").unwrap();
    assert_eq!(*first_point, Point { x: 10, y: 20 }); // Content should match
    assert_eq!(first_data, b"first"); // Content should match
    let first_point_ptr = first_point as *const Point;
    let first_data_ptr = first_data.as_ptr();

    let (get_p1, get_d1) = pool.get_assoc::<Point>(0).unwrap();
    assert_eq!(
        first_point_ptr, get_p1 as *const Point,
        "First push_assoc and get_assoc(0) should point to same Point memory"
    );
    assert_eq!(
        first_data_ptr,
        get_d1.as_ptr(),
        "First push_assoc and get_assoc(0) should point to same data memory"
    );

    let (second_point, second_data) = pool.push_assoc(Point { x: 30, y: 40 }, b"second").unwrap();
    assert_eq!(*second_point, Point { x: 30, y: 40 }); // Content should match
    assert_eq!(second_data, b"second"); // Content should match
    let second_point_ptr = second_point as *const Point;
    let second_data_ptr = second_data.as_ptr();

    let (get_p2, get_d2) = pool.get_assoc::<Point>(1).unwrap();
    assert_eq!(
        second_point_ptr, get_p2 as *const Point,
        "Second push_assoc and get_assoc(1) should point to same Point memory"
    );
    assert_eq!(
        second_data_ptr,
        get_d2.as_ptr(),
        "Second push_assoc and get_assoc(1) should point to same data memory"
    );

    let (third_point, third_data) = pool.push_assoc(Point { x: 50, y: 60 }, b"third").unwrap();
    assert_eq!(*third_point, Point { x: 50, y: 60 }); // Content should match
    assert_eq!(third_data, b"third"); // Content should match
    let third_point_ptr = third_point as *const Point;
    let third_data_ptr = third_data.as_ptr();

    let (get_p3, get_d3) = pool.get_assoc::<Point>(2).unwrap();
    assert_eq!(
        third_point_ptr, get_p3 as *const Point,
        "Third push_assoc and get_assoc(2) should point to same Point memory"
    );
    assert_eq!(
        third_data_ptr,
        get_d3.as_ptr(),
        "Third push_assoc and get_assoc(2) should point to same data memory"
    );

    // Verify each push_assoc uses different memory locations (independence)
    assert_ne!(first_point_ptr, second_point_ptr);
    assert_ne!(second_point_ptr, third_point_ptr);
    assert_ne!(first_point_ptr, third_point_ptr);
    assert_ne!(first_data_ptr, second_data_ptr);
    assert_ne!(second_data_ptr, third_data_ptr);
    assert_ne!(first_data_ptr, third_data_ptr);
}

#[test]
fn test_push_assoc_return_value_with_empty_data() {
    let mut buffer = [0u8; 600];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test pushing with empty data slice
    let point = Point { x: 123, y: 456 };
    let (stored_point, stored_data) = pool.push_assoc(point, b"").unwrap();

    assert_eq!(*stored_point, point); // Content should match
    assert_eq!(stored_data, b""); // Content should match
    assert_eq!(stored_data.len(), 0);

    // Verify push_assoc() and get_assoc() point to the same memory locations even for empty data
    let stored_point_ptr = stored_point as *const Point;
    let stored_data_ptr = stored_data.as_ptr();
    let stored_data_len = stored_data.len();
    let _ = (stored_point, stored_data);

    let (get_point, get_data) = pool.get_assoc::<Point>(0).unwrap();
    let get_point_ptr = get_point as *const Point;
    let get_data_ptr = get_data.as_ptr();
    let get_data_len = get_data.len();

    assert_eq!(
        stored_point_ptr, get_point_ptr,
        "push_assoc() and get_assoc() should point to same Point memory for empty data"
    );
    assert_eq!(
        stored_data_ptr, get_data_ptr,
        "push_assoc() and get_assoc() should point to same data memory for empty data"
    );
    assert_eq!(stored_data_len, get_data_len);
}

#[test]
fn test_push_assoc_pointer_equality_across_different_associated_types() {
    let mut buffer = [0u8; 600];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test with different associated value types
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct SimpleInt {
        value: i32,
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct BigStruct {
        a: u64,
        b: u64,
        c: u32,
    }

    let simple = SimpleInt { value: 42 };
    let big = BigStruct {
        a: 0x1111111111111111,
        b: 0x2222222222222222,
        c: 0x33333333,
    };

    let (simple_ref, simple_data) = pool.push_assoc(simple, b"simple data").unwrap();
    assert_eq!(*simple_ref, simple); // Content should match
    assert_eq!(simple_data, b"simple data"); // Content should match
    let simple_ptr = simple_ref as *const SimpleInt;
    let simple_data_ptr = simple_data.as_ptr();

    let (big_ref, big_data) = pool.push_assoc(big, b"big data").unwrap();
    assert_eq!(*big_ref, big); // Content should match
    assert_eq!(big_data, b"big data"); // Content should match
    let big_ptr = big_ref as *const BigStruct;
    let big_data_ptr = big_data.as_ptr();

    // Verify pointer equality with get_assoc results for different types
    let (get_simple, get_simple_data) = pool.get_assoc::<SimpleInt>(0).unwrap();
    let (get_big, get_big_data) = pool.get_assoc::<BigStruct>(1).unwrap();

    assert_eq!(
        simple_ptr, get_simple as *const SimpleInt,
        "SimpleInt: push_assoc and get_assoc should point to same memory"
    );
    assert_eq!(
        simple_data_ptr,
        get_simple_data.as_ptr(),
        "SimpleInt data: push_assoc and get_assoc should point to same memory"
    );
    assert_eq!(
        big_ptr, get_big as *const BigStruct,
        "BigStruct: push_assoc and get_assoc should point to same memory"
    );
    assert_eq!(
        big_data_ptr,
        get_big_data.as_ptr(),
        "BigStruct data: push_assoc and get_assoc should point to same memory"
    );

    assert_eq!(*get_simple, simple);
    assert_eq!(get_simple_data, b"simple data");
    assert_eq!(*get_big, big);
    assert_eq!(get_big_data, b"big data");
}

#[test]
fn test_push_assoc_pointer_equality_across_different_data_sizes() {
    let mut buffer = [0u8; 600];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Test with various data sizes
    let point1 = Point { x: 10, y: 20 };
    let point2 = Point { x: 30, y: 40 };
    let point3 = Point { x: 50, y: 60 };

    let small_data = b"x";
    let medium_data = b"hello world test data";
    let large_data = vec![b'A'; 100];

    let (small_point, small_data_ref) = pool.push_assoc(point1, small_data).unwrap();
    assert_eq!(*small_point, point1); // Content should match
    assert_eq!(small_data_ref, small_data); // Content should match
    let small_point_ptr = small_point as *const Point;
    let small_data_ptr = small_data_ref.as_ptr();

    let (medium_point, medium_data_ref) = pool.push_assoc(point2, medium_data).unwrap();
    assert_eq!(*medium_point, point2); // Content should match
    assert_eq!(medium_data_ref, medium_data); // Content should match
    let medium_point_ptr = medium_point as *const Point;
    let medium_data_ptr = medium_data_ref.as_ptr();

    let (large_point, large_data_ref) = pool.push_assoc(point3, &large_data).unwrap();
    assert_eq!(*large_point, point3); // Content should match
    assert_eq!(large_data_ref, &large_data[..]); // Content should match
    let large_point_ptr = large_point as *const Point;
    let large_data_ptr = large_data_ref.as_ptr();

    // Verify pointer equality with get_assoc() for various data sizes
    let (get_p1, get_d1) = pool.get_assoc::<Point>(0).unwrap();
    let (get_p2, get_d2) = pool.get_assoc::<Point>(1).unwrap();
    let (get_p3, get_d3) = pool.get_assoc::<Point>(2).unwrap();

    assert_eq!(
        small_point_ptr, get_p1 as *const Point,
        "Small Point: push_assoc and get_assoc(0) should point to same memory"
    );
    assert_eq!(
        small_data_ptr,
        get_d1.as_ptr(),
        "Small data: push_assoc and get_assoc(0) should point to same memory"
    );
    assert_eq!(
        medium_point_ptr, get_p2 as *const Point,
        "Medium Point: push_assoc and get_assoc(1) should point to same memory"
    );
    assert_eq!(
        medium_data_ptr,
        get_d2.as_ptr(),
        "Medium data: push_assoc and get_assoc(1) should point to same memory"
    );
    assert_eq!(
        large_point_ptr, get_p3 as *const Point,
        "Large Point: push_assoc and get_assoc(2) should point to same memory"
    );
    assert_eq!(
        large_data_ptr,
        get_d3.as_ptr(),
        "Large data: push_assoc and get_assoc(2) should point to same memory"
    );

    // Verify all entries use different memory locations (independence)
    assert_ne!(small_point_ptr, medium_point_ptr);
    assert_ne!(medium_point_ptr, large_point_ptr);
    assert_ne!(small_point_ptr, large_point_ptr);
    assert_ne!(small_data_ptr, medium_data_ptr);
    assert_ne!(medium_data_ptr, large_data_ptr);
    assert_ne!(small_data_ptr, large_data_ptr);
}

#[test]
fn test_push_assoc_return_value_with_alignment_requirements() {
    let mut buffer = [0u8; 600];
    let mut pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

    // Create misalignment by pushing an odd-sized regular slice first
    pool.push(b"odd").unwrap(); // 3 bytes

    // Define types with different alignment requirements
    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Aligned8 {
        value: u64,
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct Aligned4 {
        value: u32,
    }

    let aligned8_val = Aligned8 {
        value: 0x123456789ABCDEF0,
    };
    let aligned4_val = Aligned4 { value: 0x12345678 };

    // Push aligned types and verify return values
    let (stored_8, data_8) = pool.push_assoc(aligned8_val, b"eight").unwrap();
    assert_eq!(*stored_8, aligned8_val); // Content should match
    assert_eq!(data_8, b"eight"); // Content should match

    // Verify alignment by checking pointer addresses
    let ptr_8 = stored_8 as *const Aligned8 as usize;
    assert_eq!(ptr_8 % 8, 0, "Aligned8 should be 8-byte aligned");
    let stored_8_ptr = stored_8 as *const Aligned8;
    let stored_8_data_ptr = data_8.as_ptr();

    let (stored_4, data_4) = pool.push_assoc(aligned4_val, b"four").unwrap();
    assert_eq!(*stored_4, aligned4_val); // Content should match
    assert_eq!(data_4, b"four"); // Content should match

    let ptr_4 = stored_4 as *const Aligned4 as usize;
    assert_eq!(ptr_4 % 4, 0, "Aligned4 should be 4-byte aligned");
    let stored_4_ptr = stored_4 as *const Aligned4;
    let stored_4_data_ptr = data_4.as_ptr();

    // Verify pointer equality with get_assoc() even with alignment requirements
    let (get_8, get_data_8) = pool.get_assoc::<Aligned8>(1).unwrap();
    let (get_4, get_data_4) = pool.get_assoc::<Aligned4>(2).unwrap();

    assert_eq!(
        stored_8_ptr, get_8 as *const Aligned8,
        "Aligned8: push_assoc and get_assoc(1) should point to same memory"
    );
    assert_eq!(
        stored_8_data_ptr,
        get_data_8.as_ptr(),
        "Aligned8 data: push_assoc and get_assoc(1) should point to same memory"
    );
    assert_eq!(
        stored_4_ptr, get_4 as *const Aligned4,
        "Aligned4: push_assoc and get_assoc(2) should point to same memory"
    );
    assert_eq!(
        stored_4_data_ptr,
        get_data_4.as_ptr(),
        "Aligned4 data: push_assoc and get_assoc(2) should point to same memory"
    );

    assert_eq!(*get_8, aligned8_val);
    assert_eq!(get_data_8, b"eight");
    assert_eq!(*get_4, aligned4_val);
    assert_eq!(get_data_4, b"four");
}
