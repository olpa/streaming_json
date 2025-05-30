use std::io::Cursor;
use std::sync::Arc;

use rjiter::jiter::{JsonValue, LazyIndexMap, NumberInt, Peek};
use rjiter::RJiter;
use rjiter::Result as RJiterResult;
mod one_byte_reader;
use crate::one_byte_reader::OneByteReader;
mod chunk_reader;
use crate::chunk_reader::ChunkReader;

#[test]
fn sanity_check() {
    let input = r#"{}"#;
    let mut buffer = [0u8; 16];
    let mut reader = Cursor::new(input.as_bytes());

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_value();
    assert!(result.is_ok());

    let empty_object = JsonValue::Object(Arc::new(LazyIndexMap::new()));
    assert_eq!(result.unwrap(), empty_object);
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

#[test]
fn jiter_doc_example() {
    let json_data = r#"
    {
        "name": "John Doe", 
        "age": 43,
        "phones": [
            "+44 1234567",
            "+44 2345678"
        ]
    }"#;
    let mut buffer = [0u8; 16];
    let mut reader = Cursor::new(json_data.as_bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    assert_eq!(rjiter.next_object().unwrap(), Some("name"));
    assert_eq!(rjiter.next_str().unwrap(), "John Doe");
    assert_eq!(rjiter.next_key().unwrap(), Some("age"));
    assert_eq!(rjiter.next_int().unwrap(), NumberInt::Int(43));
    assert_eq!(rjiter.next_key().unwrap(), Some("phones"));
    assert_eq!(rjiter.next_array().unwrap(), Some(Peek::String));
    // we know the next value is a string as we just asserted so
    assert_eq!(rjiter.known_str().unwrap(), "+44 1234567");
    assert_eq!(rjiter.array_step().unwrap(), Some(Peek::String));
    // same again
    assert_eq!(rjiter.known_str().unwrap(), "+44 2345678");
    // next we'll get `None` from `array_step` as the array is finished
    assert_eq!(rjiter.array_step().unwrap(), None);
    // and `None` from `next_key` as the object is finished
    assert_eq!(rjiter.next_key().unwrap(), None);
    // and we check there's nothing else in the input
    rjiter.finish().unwrap();
}

//
// Pass-through long strings
//

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
fn pass_through_long_string() {
    let input = r#""very very very long string""#;
    let mut buffer = [0u8; 5];
    let mut reader = OneByteReader::new(input.bytes());
    let mut writer = Vec::new();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let wb = rjiter.write_long_str(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "very very very long string".as_bytes());
}

#[test]
fn regression_pass_through_long_string_with_chunk_reader() {
    let input = r#""very very very long string""#;
    let mut buffer = [0u8; 5];
    let mut reader = Cursor::new(input.as_bytes());
    let mut writer = Vec::new();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let wb = rjiter.write_long_str(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "very very very long string".as_bytes());
}

#[test]
fn write_long_with_unicode_code_point_on_border() {
    let input = r#""Viele Grüße""#;
    for buf_len in input.len()..input.len() + 10 {
        // Test write_long_bytes
        {
            let mut buffer = vec![0u8; buf_len];
            let mut reader = OneByteReader::new(input.bytes());
            let mut writer = Vec::new();
            let mut rjiter = RJiter::new(&mut reader, &mut buffer);

            let wb = rjiter.write_long_bytes(&mut writer);
            wb.unwrap();

            assert_eq!(writer, "Viele Grüße".as_bytes());
        }

        // Test write_long_str
        {
            let mut buffer = vec![0u8; buf_len];
            let mut reader = OneByteReader::new(input.bytes());
            let mut writer = Vec::new();
            let mut rjiter = RJiter::new(&mut reader, &mut buffer);

            let wb = rjiter.write_long_str(&mut writer);
            wb.unwrap();

            assert_eq!(writer, "Viele Grüße".as_bytes());
        }
    }
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
fn long_write_regression_segment_from_quote() {
    let input = r#"      "bar" true"#;
    let buf_len = input.find("a").unwrap();
    let mut buffer = vec![0u8; buf_len];
    let mut reader = Cursor::new(input.as_bytes());
    let mut writer = Vec::new();
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);
    rjiter.finish().unwrap_err();

    let wb = rjiter.write_long_bytes(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "bar".as_bytes());

    let after_bar = rjiter.peek().unwrap();
    assert_eq!(after_bar, Peek::True);
}

#[test]
fn long_write_regression_quote_last_buffer_byte() {
    let input = r#"      "bar" true"#;
    let buf_len = input.find("b").unwrap();
    let mut buffer = vec![0u8; buf_len];
    let mut reader = Cursor::new(input.as_bytes());
    let mut writer = Vec::new();
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);
    rjiter.finish().unwrap_err();

    let wb = rjiter.write_long_bytes(&mut writer);
    wb.unwrap();

    assert_eq!(writer, "bar".as_bytes());

    let after_bar = rjiter.peek().unwrap();
    assert_eq!(after_bar, Peek::True);
}

#[test]
fn write_long_with_bs_in_first_position() {
    let input = r#""\\ how can I help you?""#;

    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(input.as_bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let mut writer = Vec::new();
    let wb = rjiter.write_long_str(&mut writer);
    wb.unwrap();
    assert_eq!(writer, "\\ how can I help you?".as_bytes());
}

#[test]
fn write_long_with_unicode_bs_in_first_position() {
    let input = r#""\u4F60\u597d, how can I help you?""#;

    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(input.as_bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let mut writer = Vec::new();
    let wb = rjiter.write_long_str(&mut writer);
    wb.unwrap();
    assert_eq!(writer, "\u{4F60}\u{597d}, how can I help you?".as_bytes());
}

//
// Next key
//

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

//
// `finish()`
//

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
fn handle_buffer_end_pos_in_finish() {
    let input = r#"true  }  false"#;
    let pos = input.find("}").unwrap();
    let mut buffer = vec![0u8; pos + 1];
    let mut reader = Cursor::new(input);
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Move the jiter position to the end of buffer
    let result = rjiter.next_bool();
    assert_eq!(result.unwrap(), true);
    let result = rjiter.next_key();
    assert_eq!(result.unwrap(), None);
    assert_eq!(rjiter.current_index(), pos + 1);

    // Act and assert: not finished
    let result = rjiter.finish();
    assert!(result.is_err());
}

//
// Skip token
//

#[test]
fn known_skip_token() {
    let n_spaces = 6;
    let some_spaces = " ".repeat(n_spaces);
    let input = format!(r#"{some_spaces}trux true"#);
    for buffer_len in n_spaces..input.len() {
        let mut buffer = vec![0u8; buffer_len];
        let mut reader = Cursor::new(input.as_bytes());
        let mut rjiter = RJiter::new(&mut reader, &mut buffer);

        // Position Jiter on the token
        let _ = rjiter.peek();

        // Consume the "trux" token
        let result = rjiter.known_skip_token(b"trux");
        assert!(result.is_ok(), "skip_token failed");

        // The Jiter position should be moved to the "true" token
        let result = rjiter.peek();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Peek::True);

        // Do not consume the "trux" token on "true"
        let result = rjiter.known_skip_token(b"trux");
        assert!(result.is_err());

        // Consume the "true" token
        let result = rjiter.next_bool();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }
}

#[test]
fn skip_tokens_example_for_readme() {
    let json_data = r#"
        event: ping
        data: {"type": "ping"}
    "#;

    fn peek_skipping_tokens(rjiter: &mut RJiter, tokens: &[&str]) -> RJiterResult<Peek> {
        'outer: loop {
            let peek = rjiter.peek();
            for token in tokens {
                let found = rjiter.known_skip_token(token.as_bytes());
                if found.is_ok() {
                    continue 'outer;
                }
            }
            return peek;
        }
    }

    let mut buffer = [0u8; 10];
    let mut reader = Cursor::new(json_data.as_bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let tokens = vec!["data:", "event:", "ping"];
    let result = peek_skipping_tokens(&mut rjiter, &tokens);
    assert_eq!(result.unwrap(), Peek::Object);

    let key = rjiter.next_object();
    assert_eq!(key.unwrap(), Some("type"));
}

//
// Current index
//

#[test]
fn current_index() {
    let input = r#" data+   {  "foo":  "bar"}  "#;
    let pos_data_pre = 1;
    let pos_data_post = pos_data_pre + 5;
    let pos_key_post = input.find(":").unwrap() + 1;
    let pos_value_pre = input.find("b").unwrap() - 1;
    let pos_value_post = pos_value_pre + 3 + 2;
    let pos_object_post = input.find("}").unwrap() + 1;
    let pos_len_done = input.len();

    for buffer_len in 8..input.len() {
        let mut buffer = vec![0u8; buffer_len];
        let mut reader = Cursor::new(input.as_bytes());
        let mut rjiter = RJiter::new(&mut reader, &mut buffer);

        let result = rjiter.finish();
        assert!(result.is_err());
        assert_eq!(rjiter.current_index(), pos_data_pre);

        rjiter.known_skip_token(b"data+").unwrap();
        assert_eq!(rjiter.current_index(), pos_data_post);

        let result = rjiter.next_object();
        assert_eq!(result.unwrap(), Some("foo"));
        assert_eq!(rjiter.current_index(), pos_key_post);

        let result = rjiter.peek();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Peek::String);
        assert_eq!(rjiter.current_index(), pos_value_pre);

        let result = rjiter.write_long_str(&mut std::io::sink());
        assert!(result.is_ok());
        assert_eq!(rjiter.current_index(), pos_value_post);

        let result = rjiter.next_key();
        assert_eq!(result.unwrap(), None);
        assert_eq!(rjiter.current_index(), pos_object_post);

        let result = rjiter.finish();
        assert!(result.is_ok());
        assert_eq!(rjiter.current_index(), pos_len_done);
    }
}

//
// Regression tests
//

#[test]
fn regression_next_value_empty_object_with_extra_bracket() {
    let input = r#"{}}"#; // extra bracket
    let mut buffer = [0u8; 16];
    let mut reader = Cursor::new(input.as_bytes());

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.next_value();
    assert!(result.is_ok());

    let empty_object = JsonValue::Object(Arc::new(LazyIndexMap::new()));
    assert_eq!(result.unwrap(), empty_object);
}

#[test]
fn regression_oversize_string_with_long_unicode_code_point() {
    let input = r#""AAA\n├AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA""#;
    let mut buffer = [0u8; 16];
    let mut reader = Cursor::new(input.as_bytes());
    let mut writer = Vec::new();
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let wb = rjiter.write_long_str(&mut writer);
    wb.unwrap();

    assert_eq!(
        writer,
        "AAA\n\u{251c}AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".as_bytes()
    );
}

#[test]
fn regression_long_writer_search_escape_in_nbytes() {
    let input_str = r#""123@456""#;
    let input = input_str.as_bytes().to_vec();
    let mut buffer = [b'A', b'A', b'A', b'A', b'A', b'A', b'\\', b'n'];

    let mut reader = ChunkReader::new(&input, b'@');
    let mut writer = Vec::new();
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Act
    let wb = rjiter.write_long_str(&mut writer);
    wb.unwrap();

    // Assert
    // Error was: the code searched for an escape in the whole buffer instead
    // of limiting to `n_bytes`, so that the result was 'AAAAA123AA456'
    assert_eq!(writer, "123456".as_bytes());
}

#[test]
fn regression_long_writer_search_escape_in_nbytes_2() {
    // Like `regression_long_writer_search_escape_in_nbytes`,
    // but have the escape immediately after the n_bytes
    let input = r#""123456""#;
    let mut buffer = [b'"', b'*', b'\\', b'n', b'*', b'*', b'*', b'*'];

    let mut reader = OneByteReader::new(input.bytes());
    let mut writer = Vec::new();
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Act
    let wb = rjiter.write_long_str(&mut writer);
    wb.unwrap();

    // Assert
    assert_eq!(writer, "123456".as_bytes());
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
fn next_number() {
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
