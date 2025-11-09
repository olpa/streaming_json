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
    let mut reader = input.as_bytes();

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
    let mut reader = json_data.as_bytes();
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
    let mut reader = input;

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
    let mut reader = input.as_bytes();
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
    let mut reader = input.as_bytes();
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
    let mut reader = input.as_bytes();
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
    let mut reader = input.as_bytes();
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
    let mut reader = input.as_bytes();
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
    let mut reader = input.as_bytes();
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
    let mut reader = input.as_bytes();

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
    let mut reader = input;
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    let result = rjiter.finish();
    assert!(result.is_ok());
}

#[test]
fn finish_no_when_in_buffer() {
    let input = "    x".as_bytes();
    let mut buffer = [0u8; 10];
    let mut reader = input;
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
    let mut reader = input.as_bytes();
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
        let mut reader = input.as_bytes();
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

    fn peek_skipping_tokens<R: embedded_io::Read>(
        rjiter: &mut RJiter<R>,
        tokens: &[&str],
    ) -> RJiterResult<Peek> {
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
    let mut reader = json_data.as_bytes();
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
        let mut reader = input.as_bytes();
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

        let mut sink = Vec::new();
        let result = rjiter.write_long_str(&mut sink);
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
    let mut reader = input.as_bytes();

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
    let mut reader = input.as_bytes();
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

//
// lookahead_while tests
//

#[test]
fn test_lookahead_while_without_shift() {
    let input = "12345abc";
    let mut buffer = [0u8; 16];
    let mut reader = input.as_bytes();
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Record the position before lookahead
    let pos_before = rjiter.current_index();

    // Lookahead for digits
    let result = rjiter.lookahead_while(|b| b.is_ascii_digit());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), b"12345");

    // Verify that the position hasn't changed (lookahead doesn't consume)
    let pos_after = rjiter.current_index();
    assert_eq!(pos_before, pos_after, "Position changed after lookahead");

    // Verify that peek still returns the first character
    let peek_result = rjiter.peek();
    assert!(peek_result.is_ok());
    assert_eq!(peek_result.unwrap(), Peek::new(b'1'));

    // Position should still be unchanged after peek
    assert_eq!(rjiter.current_index(), pos_before);
}

#[test]
fn test_lookahead_while_with_shift() {
    let input = "   12345abc";
    let mut buffer = [0u8; 16];
    let mut reader = input.as_bytes();
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Skip the spaces first by peeking past them
    let _ = rjiter.peek(); // This will internally handle spaces

    // Record the position before lookahead (after spaces have been skipped)
    let pos_before = rjiter.current_index();

    // Now lookahead for digits
    let result = rjiter.lookahead_while(|b| b.is_ascii_digit());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), b"12345");

    // Verify that the position hasn't changed after lookahead
    let pos_after = rjiter.current_index();
    assert_eq!(pos_before, pos_after, "Position changed after lookahead with shift");

    // Verify that we can still peek at the current position
    let peek_result = rjiter.peek();
    assert!(peek_result.is_ok());
    assert_eq!(peek_result.unwrap(), Peek::new(b'1'));

    // Position should still be unchanged after peek
    assert_eq!(rjiter.current_index(), pos_before);
}

#[test]
fn test_lookahead_while_buffer_full() {
    // Create input with many digits that exceed buffer size
    let input = "123456789012345678901234567890abc";
    let mut buffer = [0u8; 4]; // Small buffer
    let mut reader = input.as_bytes();
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Try to lookahead - should fail with BufferFull since allow_shift is false
    let result = rjiter.lookahead_while(|b| b.is_ascii_digit());
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.error_type, rjiter::error::ErrorType::BufferFull);
}

#[test]
fn test_lookahead_while_with_buffer_read() {
    // Test case where lookahead needs to read more data from the reader
    // This tests the bug where start_pos becomes invalid after buffer changes

    // Start with some JSON that will position us mid-buffer, then lookahead
    let input = r#"{"key":"value","num":12345}"#;
    let mut buffer = [0u8; 20];  // Buffer large enough to hold the lookahead result
    let mut reader = OneByteReader::new(input.bytes());
    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Parse the object to advance into the buffer
    assert_eq!(rjiter.next_object().unwrap(), Some("key"));
    assert_eq!(rjiter.next_str().unwrap(), "value");
    assert_eq!(rjiter.next_key().unwrap(), Some("num"));

    // Now we're positioned at the number. The buffer has been read and possibly shifted.
    // Record position before lookahead
    let pos_before = rjiter.current_index();

    // Lookahead for digits - this may trigger reads that change buffer.n_bytes
    // and cause create_new_jiter() to be called
    let result = rjiter.lookahead_while(|b| b.is_ascii_digit());
    assert!(result.is_ok());

    // This should return all digits
    let digits = result.unwrap();
    assert_eq!(digits, b"12345", "Lookahead should return all digits");

    // Verify position is unchanged
    let pos_after = rjiter.current_index();
    assert_eq!(pos_before, pos_after, "Position changed after lookahead");

    // Verify we can still read the number correctly
    let int_result = rjiter.next_int();
    assert!(int_result.is_ok());
    assert_eq!(int_result.unwrap(), rjiter::jiter::NumberInt::Int(12345));
}

//
// lookahead_n tests
//

/// Test 1: Normal get - lookahead n bytes that are already in buffer
#[test]
fn test_lookahead_n_normal_get() {
    let input = b"1234567890abcdef";
    let mut buffer = [0u8; 32];
    let mut reader = input.as_slice();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Lookahead 5 bytes
    let result = rjiter.lookahead_n(5);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec(); // Copy to avoid borrow issues
    assert_eq!(bytes, b"12345");

    // Lookahead should not consume - we can lookahead again
    let result = rjiter.lookahead_n(3);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec();
    assert_eq!(bytes, b"123");

    // Lookahead larger amount
    let result = rjiter.lookahead_n(10);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec();
    assert_eq!(bytes, b"1234567890");
}

/// Test 2: Buffer too small - request more bytes than buffer can hold
#[test]
fn test_lookahead_n_buffer_too_small() {
    let input = b"1234567890abcdef";
    let mut buffer = [0u8; 8]; // Small buffer
    let mut reader = input.as_slice();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Try to lookahead more bytes than buffer can hold
    let result = rjiter.lookahead_n(20);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, rjiter::error::ErrorType::BufferFull);
}

/// Test 3: Get to EOF, less than n - request more bytes than available
#[test]
fn test_lookahead_n_eof_less_than_n() {
    let input = b"12345";
    let mut buffer = [0u8; 32];
    let mut reader = input.as_slice();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Try to lookahead more bytes than available
    let result = rjiter.lookahead_n(10);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec();
    // Should return only what's available (5 bytes)
    assert_eq!(bytes, b"12345");
    assert_eq!(bytes.len(), 5);
}

/// Test 4: Shift in collect_count - buffer needs to shift to make room
#[test]
fn test_lookahead_n_shift_in_collect() {
    let input = b"false1234567890abcdefghij";
    let mut buffer = [0u8; 12]; // Small buffer to force shifting
    let mut reader = OneByteReader::new(input.iter().copied());

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // First, consume the "false" token to move the jiter position forward
    let result = rjiter.next_bool();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);

    // Now we're at position 5 (after "false")
    // The buffer has limited space, so requesting many bytes should trigger shift
    let result = rjiter.lookahead_n(8);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec();
    // Should successfully get 8 bytes starting from current position
    assert_eq!(bytes, b"12345678");
}

/// Test 5: Read in collect_count - needs to read more data from reader
#[test]
fn test_lookahead_n_read_in_collect() {
    // Use ChunkReader to control when data becomes available
    let data = b"1234567890abcdefghijklmnop".to_vec();
    let mut buffer = [0u8; 32];
    // ChunkReader with interrupt at 'f' - splits data into chunks
    let mut reader = ChunkReader::new(&data, b'f');

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Request 15 bytes - should require reading across the chunk boundary
    let result = rjiter.lookahead_n(15);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec();
    assert_eq!(bytes, b"1234567890abcde");
}

/// Test 6: Lookahead after consuming some data
#[test]
fn test_lookahead_n_after_consume() {
    let input = br#"{"key":"value"}"#;
    let mut buffer = [0u8; 32];
    let mut reader = input.as_slice();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Consume the opening brace
    let obj = rjiter.next_object().unwrap();
    assert_eq!(obj, Some("key"));

    // Now lookahead at the value
    let result = rjiter.lookahead_n(7);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec();
    assert_eq!(bytes, b"\"value\"");
}

/// Test 7: Lookahead zero bytes
#[test]
fn test_lookahead_n_zero_bytes() {
    let input = b"1234567890";
    let mut buffer = [0u8; 16];
    let mut reader = input.as_slice();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Lookahead 0 bytes
    let result = rjiter.lookahead_n(0);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec();
    assert_eq!(bytes.len(), 0);
}

/// Test 8: Multiple lookaheads with different sizes
#[test]
fn test_lookahead_n_multiple_sizes() {
    let input = b"abcdefghijklmnop";
    let mut buffer = [0u8; 32];
    let mut reader = input.as_slice();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // First lookahead
    {
        let bytes = rjiter.lookahead_n(3).unwrap();
        assert_eq!(bytes, b"abc");
    }

    // Second lookahead - larger
    {
        let bytes = rjiter.lookahead_n(7).unwrap();
        assert_eq!(bytes, b"abcdefg");
    }

    // Third lookahead - smaller again
    {
        let bytes = rjiter.lookahead_n(2).unwrap();
        assert_eq!(bytes, b"ab");
    }
}

/// Test 9: Lookahead with OneByteReader (forces multiple reads)
#[test]
fn test_lookahead_n_one_byte_reader() {
    let input = b"The quick brown fox";
    let mut buffer = [0u8; 32];
    let mut reader = OneByteReader::new(input.iter().copied());

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Request 10 bytes - OneByteReader only reads 1 byte at a time
    let result = rjiter.lookahead_n(10);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec();
    assert_eq!(bytes, b"The quick ");
    assert_eq!(bytes.len(), 10);
}

/// Test 10: Lookahead exact buffer size
#[test]
fn test_lookahead_n_exact_buffer_size() {
    let input = b"1234567890abcdefghij";
    let mut buffer = [0u8; 10];
    let mut reader = input.as_slice();

    let mut rjiter = RJiter::new(&mut reader, &mut buffer);

    // Request exact buffer size
    let result = rjiter.lookahead_n(10);
    assert!(result.is_ok());
    let bytes = result.unwrap().to_vec();
    assert_eq!(bytes, b"1234567890");
}
