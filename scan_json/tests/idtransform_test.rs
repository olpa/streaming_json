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
