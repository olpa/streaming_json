use rjiter::RJiter;
use scan_json::idtransform::idtransform;
use std::cell::RefCell;

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
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer).unwrap();
    let output = String::from_utf8(writer).unwrap();
    let expected = input.split_whitespace().collect::<Vec<&str>>().join(" ");
    assert_eq!(
        output.trim(),
        expected,
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
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer).unwrap();
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
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer).unwrap();
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
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer).unwrap();
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
                "street": "123 Main St",
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
            }
        }

        {"x": [{}, {}, [[],[]]]}
    "#;

    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 32];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut writer = Vec::new();

    //
    // Apply and assert
    //
    idtransform(&rjiter_cell, &mut writer).unwrap();
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
