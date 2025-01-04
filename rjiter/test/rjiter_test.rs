use std::io::Cursor;
use std::sync::Arc;

use gpt::rjiter::RJiter;
use jiter::JsonValue;
use jiter::LazyIndexMap;
use jiter::Peek;

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
fn pass_through_long_string() {
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
    let wb = rjiter.write_bytes(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "very very very long string".as_bytes());
}

#[test]
fn pass_through_small_string() {
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
    let wb = rjiter.write_bytes(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "small".as_bytes());
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
