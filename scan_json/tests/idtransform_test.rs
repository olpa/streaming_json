use embedded_io::Write;
use rjiter::RJiter;
use scan_json::idtransform::idtransform;
use std::cell::RefCell;
use u8pool::U8Pool;

#[test]
fn idt_atomic_on_top() {
    let input = r#"
        null
        true false
        42 3.14
        "hello"
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    for _ in 0..input.split_whitespace().count() {
        idtransform(&rjiter_cell, &mut writer, &mut scan_stack).unwrap();
        writer.write_all(b" ").unwrap();
    }
    let output = String::from_utf8(writer).unwrap();
    let output = output.trim();
    let expected = input.split_whitespace().collect::<Vec<&str>>().join(" ");
    assert_eq!(
        output, expected,
        "Output should match input after idtransform. Output: {output}"
    );
}

#[test]
fn idt_atomic_in_object() {
    let input = r#"
        {
            "null": null,
            "bool": true,
            "number": 42,
            "float": 3.14,
            "string": "hello"
        }
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer, &mut scan_stack).unwrap();
    let output = String::from_utf8(writer).unwrap();
    let expected = input.split_whitespace().collect::<Vec<&str>>().join("");
    assert_eq!(
        output.trim(),
        expected,
        "Output should match input after idtransform. Output: {output}"
    );
}

#[test]
fn idt_atomic_in_array() {
    let input = r#"
        [
            null,
            true,
            42,
            3.14,
            "hello"
        ]
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer, &mut scan_stack).unwrap();
    let output = String::from_utf8(writer).unwrap();
    let expected = input.split_whitespace().collect::<Vec<&str>>().join("");
    assert_eq!(
        output.trim(),
        expected,
        "Output should match input after idtransform. Output: {output}"
    );
}

#[test]
fn idt_object_in_object() {
    let input = r#"{ "foo": { "bar": "baz" } }"#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer, &mut scan_stack).unwrap();
    let output = String::from_utf8(writer).unwrap();
    let expected = input.split_whitespace().collect::<Vec<&str>>().join("");
    assert_eq!(
        output.trim(),
        expected,
        "Output should match input after idtransform. Output: {output}"
    );
}

#[test]
fn idt_array_in_array() {
    let input = r#"
        [
            [1, 2, 3],
            ["a", "b", "c"],
            [true, false, null]
        ]
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer, &mut scan_stack).unwrap();
    let output = String::from_utf8(writer).unwrap();
    let expected = input.split_whitespace().collect::<Vec<&str>>().join("");
    assert_eq!(
        output.trim(),
        expected,
        "Output should match input after idtransform. Output: {output}"
    );
}

#[test]
fn idt_deeply_nested() {
    let input = r#"
        {
            "name": "John",
            "age": 42,
            "address": {
                "street": "123_Main_St",
                "city": "Anytown",
                "country": "USA",
                "coordinates": {
                    "lat": 40.7128,
                    "long": -74.0060
                }
            },
            "contacts": [
                {
                    "type": "email",
                    "value": "john@example.com",
                    "tags": ["personal", "primary"]
                },
                {
                    "type": "phone",
                    "value": "+1-555-123-4567",
                    "tags": ["work"],
                    "hours": {
                        "mon-fri": "9-5",
                        "sat-sun": "closed"
                    }
                }
            ],
            "active": true,
            "preferences": {
                "notifications": {
                    "email": true,
                    "sms": false,
                    "frequency": {
                        "marketing": "weekly",
                        "updates": "daily"
                    }
                }
            },
            "x": [{}, {}, [[],[]]]
        }
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 32];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer, &mut scan_stack).unwrap();
    let output = String::from_utf8(writer).unwrap();
    let expected = input
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("")
        .replace("}{\"x", "} {\"x");
    assert_eq!(
        output.trim(),
        expected,
        "Output should match input after idtransform. Output: {output}"
    );
}

#[test]
fn idt_long_strings() {
    let input = r#"
        {
            "long_key": "this_is_a_much_longer_string_value_that_spans_multiple_words_and_includes_punctuation!",
            "array": [
                "another_lengthy_string_value_in_the_array_context_that_spans_multiple_words_and_includes_punctuation!"
            ]
        }
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let mut writer = Vec::new();

    idtransform(&rjiter_cell, &mut writer, &mut scan_stack).unwrap();

    let output = String::from_utf8(writer).unwrap();
    let expected = input.split_whitespace().collect::<Vec<&str>>().join("");
    assert_eq!(
        output.trim(),
        expected,
        "Output should match input after idtransform. Output: {output}"
    );
}

#[test]
fn idt_special_symbols() {
    let input = r#"
        {
            "key!@#$%": "value!@#$%",
            "unicode★": "symbols★",
            "escaped\"quotes\"": "with\"quotes\"",
            "back\\slash": "with\\slash",
            "control\n\t\r": "chars\n\t\r"
        }
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 32];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let mut writer = Vec::new();

    idtransform(&rjiter_cell, &mut writer, &mut scan_stack).unwrap();

    let output = String::from_utf8(writer).unwrap();
    let expected = input.split_whitespace().collect::<Vec<&str>>().join("");
    assert_eq!(
        output.trim(),
        expected,
        "Output should match input after idtransform. Output: {output}"
    );
}

#[test]
fn idt_stop_after_object() {
    let input = r#"
        {
            "key1": "value1",
            "key2": "value2"
        }
        {
            "next_obj_key": "next_obj_value"
        }
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 32];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let mut writer = Vec::new();

    //
    // Act: Transform first object
    //
    idtransform(&rjiter_cell, &mut writer, &mut scan_stack).unwrap();

    //
    // Assert: First object should be transformed correctly
    //
    let output = String::from_utf8(writer).unwrap();
    let expected = r#"{"key1":"value1","key2":"value2"}"#;
    assert_eq!(
        output.trim(),
        expected,
        "First object should be transformed correctly. Output: {output}"
    );

    //
    // Act: RJiter should be able to read the next key-value pair
    //
    let mut rjiter = rjiter_cell.borrow_mut();
    let key = rjiter.next_object().unwrap();
    assert_eq!(key, Some("next_obj_key"));
}
