use rjiter::jiter::Peek;
use rjiter::RJiter;

mod one_byte_reader;
use crate::one_byte_reader::OneByteReader;
mod chunk_reader;
use crate::chunk_reader::ChunkReader;

//
// known_skip_token tests
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
