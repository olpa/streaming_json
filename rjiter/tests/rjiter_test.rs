use std::io::Cursor;
use std::sync::Arc;

use jiter::JsonValue;
use jiter::LazyIndexMap;
use rjiter::Peek;
use rjiter::RJiter;

mod one_byte_reader;
use crate::one_byte_reader::OneByteReader;

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
fn pass_through_small_bytes() {
    let input = r#""small text""#;
    let mut buffer = [0u8; 100];
    let mut reader = Cursor::new(input.as_bytes());
    let mut writer = Vec::new();
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let wb = rjiter.write_long_bytes(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "small text".as_bytes());
}

#[test]
fn pass_through_long_bytes() {
    let input = r#""very very very long string""#;
    let mut buffer = [0u8; 5];
    let mut reader = OneByteReader::new(input.bytes());
    let mut writer = Vec::new();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let wb = rjiter.write_long_bytes(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "very very very long string".as_bytes());
}

#[test]
fn escapes_in_pass_through_long_bytes() {
    let input = r#""escapes X\n\\\"\u0410""#;
    let pos = input.find("X").unwrap();
    for buf_len in pos..input.len() {
        let mut buffer = vec![0u8; buf_len];
        let mut reader = OneByteReader::new(input.bytes());
        let mut writer = Vec::new();
        let mut rjiter = RJiter::new(&mut reader, &mut buffer);

        let wb = rjiter.write_long_bytes(&mut writer);
        wb.unwrap();

        assert_eq!(writer, r#"escapes X\n\\\"\u0410"#.as_bytes());
    }
}

#[test]
fn pass_through_long_string_with_escapes() {
    let input = r#""I'm a very long string with escapes X\n\\\"\u0410""#;
    let pos = input.find("X").unwrap();
    for buf_len in pos..input.len() {
        let mut buffer = vec![0u8; buf_len];
        let mut reader = OneByteReader::new(input.bytes());
        let mut writer = Vec::new();
        let mut rjiter = RJiter::new(&mut reader, &mut buffer);

        let wb = rjiter.write_long_str(&mut writer);
        wb.unwrap();

        assert_eq!(
            writer,
            "I'm a very long string with escapes X\n\\\"\u{0410}".as_bytes()
        );
    }
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
fn skip_spaces_for_next_key() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces},{lot_of_spaces}"foo": "bar""#);
    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(input.as_bytes());

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // act
    let result = rjiter.next_key();

    // assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("foo"));

    // bonus assert: key value
    let result = rjiter.next_str();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "bar");
}

#[test]
fn next_key_from_one_byte_reader() {
    let input = r#" , "foo": "bar"}"#.bytes();
    let mut reader = OneByteReader::new(input);
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // act
    let result = rjiter.next_key();

    // assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("foo"));

    // bonus assert: key value
    let result = rjiter.next_str();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "bar");
}

#[test]
fn next_str_with_spaces_one_byte_reader() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // act
    let result = rjiter.next_str();

    // assert
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn finish_yes_when_in_buffer() {
    let input = "  \n\t  ".as_bytes();
    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(input);
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.finish();
    assert!(result.is_ok());
}

#[test]
fn finish_no_when_in_buffer() {
    let input = "    x".as_bytes();
    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(input);
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.finish();
    assert!(result.is_err());
}

#[test]
fn finish_yes_when_need_feed() {
    let input = " ".repeat(32);
    let mut buffer = [0u8; 10];
    let mut reader = OneByteReader::new(input.bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.finish();
    assert!(result.is_ok());
}

#[test]
fn finish_no_when_need_feed() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!("{lot_of_spaces}42");
    let mut buffer = [0u8; 10];
    let mut reader = OneByteReader::new(input.bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.finish();
    assert!(result.is_err());
}

#[test]
fn many_known_foo() {
    let input = r#"  42  "hello"  true  false  null  []  {}"#;
    let mut buffer = [0u8; 10];
    let mut reader = OneByteReader::new(input.bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.known_int(Peek::new(b'4'));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), jiter::NumberInt::Int(42));

    let result = rjiter.known_str();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");

    let result = rjiter.known_bool(Peek::True);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);

    let result = rjiter.known_bool(Peek::False);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);

    let result = rjiter.known_null();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ());

    let result = rjiter.known_array();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);

    let result = rjiter.known_object();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);

    let result = rjiter.finish();
    assert!(result.is_ok());
}

// ----------------------------------------------
// Auto-generated from a template

#[test]
fn peek() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.peek();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Peek::String);
}

#[test]
fn next_null() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}null"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_null();
    assert!(result.is_ok());
}

#[test]
fn known_null() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}null"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let peek = rjiter.peek().unwrap();
    assert_eq!(peek, Peek::Null);
    let result = rjiter.known_null();
    assert!(result.is_ok());
}

#[test]
fn next_bool() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}true"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_bool();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[test]
fn known_bool() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}false"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let peek = rjiter.peek().unwrap();
    assert_eq!(peek, Peek::False);
    let result = rjiter.known_bool(peek);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}

#[test]
fn next_number1() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}123.45"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_number();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), jiter::NumberAny::Float(123.45));
}

#[test]
fn known_number() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}123.45"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let peek = rjiter.peek().unwrap();
    assert!(peek.is_num());
    let result = rjiter.known_number(peek);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), jiter::NumberAny::Float(123.45));
}

#[test]
fn next_int() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}42"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_int();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), jiter::NumberInt::Int(42));
}

#[test]
fn known_int() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}42"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let peek = rjiter.peek().unwrap();
    assert!(peek.is_num());
    let result = rjiter.known_int(peek);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), jiter::NumberInt::Int(42));
}

#[test]
fn next_float() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}3.14"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_float();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 3.14);
}

#[test]
fn known_float() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}3.14"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let peek = rjiter.peek().unwrap();
    assert!(peek.is_num());
    let result = rjiter.known_float(peek);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 3.14);
}

#[test]
fn next_number_bytes() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}123.45"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_number_bytes();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), b"123.45");
}

#[test]
fn next_str() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_str();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn known_str() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let _ = rjiter.finish();
    let result = rjiter.known_str();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "hello");
}

#[test]
fn next_bytes() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_bytes();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), b"hello");
}

#[test]
fn known_bytes() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let _ = rjiter.finish();
    let result = rjiter.known_bytes();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), b"hello");
}

#[test]
fn next_value() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_value();
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        JsonValue::Str(std::borrow::Cow::Borrowed("hello"))
    );
}

#[test]
fn known_value() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let peek = rjiter.peek().unwrap();
    assert_eq!(peek, Peek::String);
    let result = rjiter.known_value(peek);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        JsonValue::Str(std::borrow::Cow::Borrowed("hello"))
    );
}

#[test]
fn next_skip() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello"  42"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_skip();
    assert!(result.is_ok());

    // To check that skipped, read the next value
    let result = rjiter.next_number();
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        jiter::NumberAny::Int(jiter::NumberInt::Int(42))
    );
}

#[test]
fn known_skip() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello"  42"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let peek = rjiter.peek().unwrap();
    assert_eq!(peek, Peek::String);
    let result = rjiter.known_skip(peek);
    assert!(result.is_ok());

    // To check that skipped, read the next value
    let result = rjiter.next_number();
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        jiter::NumberAny::Int(jiter::NumberInt::Int(42))
    );
}

#[test]
fn next_value_owned() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_value_owned();
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        JsonValue::Str(std::borrow::Cow::Borrowed("hello"))
    );
}

#[test]
fn known_value_owned() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}"hello""#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let peek = rjiter.peek().unwrap();
    assert_eq!(peek, Peek::String);
    let result = rjiter.known_value_owned(peek);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        JsonValue::Str(std::borrow::Cow::Borrowed("hello"))
    );
}

#[test]
fn next_array() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}[{lot_of_spaces}false{lot_of_spaces}, 2, 3]"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_array();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(Peek::False));
}

#[test]
fn known_array() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}[{lot_of_spaces}"hello"{lot_of_spaces}, 2, 3]"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let _ = rjiter.finish();
    let result = rjiter.known_array();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(Peek::String));
}

#[test]
fn array_step() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces},{lot_of_spaces} true{lot_of_spaces}, 3]"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.array_step();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(Peek::True));
}

#[test]
fn next_object() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}{{{lot_of_spaces}"key": "value"}}"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_object();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("key"));
}

#[test]
fn known_object() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}{{{lot_of_spaces}"key": "value"}}"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let _ = rjiter.finish();
    let result = rjiter.known_object();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("key"));
}

#[test]
fn next_object_bytes() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces}{{{lot_of_spaces}"key": "value"}}"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_object_bytes();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(&b"key"[..]));
}

#[test]
fn next_key_bytes() {
    let lot_of_spaces = " ".repeat(32);
    let input = format!(r#"{lot_of_spaces},{lot_of_spaces}"key": "value"}}"#);
    let mut reader = OneByteReader::new(input.bytes());
    let mut buffer = [0u8; 10];
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_key_bytes();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(&b"key"[..]));
}
