use std::io::Cursor;
use std::sync::Arc;

use jiter::JsonValue;
use jiter::LazyIndexMap;
use jiter::Peek;
use rjiter::RJiter;

#[test]
fn sanity_check() {
    let input = r#"{}}"#;
    let mut buffer = [0u8; 16];
    let mut reader = Cursor::new(input.as_bytes());

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_value();
    assert!(result.is_ok());

    let empty_object = JsonValue::Object(Arc::new(LazyIndexMap::new()));
    assert_eq!(result.unwrap(), empty_object);
}

#[test]
fn skip_spaces() {
    // Create input with 18 spaces followed by an empty JSON object
    // Use a 16-byte buffer
    let input = "               {}".as_bytes();
    let mut buffer = [0u8; 16];
    let mut reader = Cursor::new(input);

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_value();
    assert!(result.is_ok());

    let empty_object = JsonValue::Object(Arc::new(LazyIndexMap::new()));
    assert_eq!(result.unwrap(), empty_object);
}

#[test]
fn pass_through_long_bytes() {
    let input = r#"{ "text": "very very very long string" }"#;
    let mut buffer = [0u8; 10]; // Small buffer to force multiple reads
    let mut reader = Cursor::new(input.as_bytes());
    let mut writer = Vec::new();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Consume object start
    assert_eq!(rjiter.next_object().unwrap(), Some("text"));
    rjiter.feed();
    assert_eq!(rjiter.peek().unwrap(), Peek::String);

    // Consume the string value
    let wb = rjiter.write_long_bytes(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "very very very long string".as_bytes());
}

#[test]
fn pass_through_small_bytes() {
    let input = r#"{ "text": "small" }"#;
    let mut buffer = [0u8; 100];
    let mut reader = Cursor::new(input.as_bytes());
    let mut writer = Vec::new();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Consume object start
    assert_eq!(rjiter.next_object().unwrap(), Some("text"));
    rjiter.feed();
    assert_eq!(rjiter.peek().unwrap(), Peek::String);

    // Consume the string value
    let wb = rjiter.write_long_bytes(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "small".as_bytes());
}

#[test]
fn pass_through_small_string() {
    let input = r#"{ "text": "nl\ntab\tu\u0410" }"#;
    let mut buffer = [0u8; 100];
    let mut reader = Cursor::new(input.as_bytes());
    let mut writer = Vec::new();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Consume object start
    assert_eq!(rjiter.next_object().unwrap(), Some("text"));
    rjiter.feed();
    assert_eq!(rjiter.peek().unwrap(), Peek::String);

    // Consume the string value
    let wb = rjiter.write_long_str(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "nl\ntab\tu\u{0410}".as_bytes());
}

#[test]
fn pass_through_long_string() {
    let input = r#"{ "text": "very\" very\n very\u0410 long\t string\"" }"#;
    let mut buffer = [0u8; 10]; // Small buffer to force multiple reads
    let mut reader = Cursor::new(input.as_bytes());
    let mut writer = Vec::new();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Consume object start
    assert_eq!(rjiter.next_object().unwrap(), Some("text"));
    rjiter.feed();
    assert_eq!(rjiter.peek().unwrap(), Peek::String);

    // Consume the string value
    let wb = rjiter.write_long_str(&mut writer);
    wb.unwrap();

    assert_eq!(
        writer,
        "very\" very\n very\u{0410} long\t string\"".as_bytes()
    );
}

#[test]
fn skip_token() {
    let input = r#"data:  42"#;
    let mut buffer = [0u8; 16];
    let mut reader = Cursor::new(input.as_bytes());

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Consume the "data:token
    let result = rjiter.skip_token(b"data:");
    assert!(result, "skip_token failed");

    // Consume a number
    let result = rjiter.next_int();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), jiter::NumberInt::Int(42));
}

#[test]
fn multi_read_next_key() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces},{lot_of_spaces}foo": "bar""#);
    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(input.as_bytes());

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // act
    let result = rjiter.next_key();
    println!("multi_read_next_key result: {:?}", result);

    // assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("foo"));

    // bonus assert: key value
    let result = rjiter.next_str();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "bar");
}
