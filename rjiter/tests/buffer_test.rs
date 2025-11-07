use rjiter::buffer::Buffer;
use rjiter::jiter::LinePosition;

#[test]
fn test_read_until_full() {
    let input = "abcdef";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    let n_bytes = buffer.read_more().unwrap();
    assert_eq!(n_bytes, 4);

    let n_bytes = buffer.read_more().unwrap();
    assert_eq!(n_bytes, 0);
}

#[test]
fn test_basic_skip_spaces() {
    let spaces = " ".repeat(4);
    let input = format!("{spaces}abc");
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    // act
    buffer.skip_spaces(0).unwrap();

    // assert
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc");
    assert_eq!(buffer.n_shifted_out, 4);
}

#[test]
fn test_skip_spaces_from_non_zero_pos() {
    let spaces = " ".repeat(4);
    let input = format!("{spaces}abc");
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    // act
    buffer.skip_spaces(2).unwrap();

    // assert
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"  abc");
    assert_eq!(buffer.n_shifted_out, 2);
}

#[test]
fn test_skip_spaces_with_one_read() {
    let spaces = " ".repeat(5);
    let input = format!("{spaces}abc");
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // act
    buffer.skip_spaces(0).unwrap();

    // assert
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc");
    assert_eq!(buffer.n_shifted_out, 5);
}

#[test]
fn test_skip_spaces_with_many_reads_and_nonzero_pos() {
    let spaces = " ".repeat(19);
    let input = format!("{spaces}abc");
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // act
    buffer.skip_spaces(2).unwrap();

    // assert
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"  a");
    assert_eq!(buffer.n_shifted_out, 17);
}

#[test]
fn test_skip_spaces_eof_without_non_space() {
    let input = " ".repeat(5);
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // act
    buffer.skip_spaces(0).unwrap();

    // assert
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"");
    assert_eq!(buffer.n_shifted_out, 5);
}

#[test]
fn test_skip_spaces_eof_without_non_space_and_nonzero_pos() {
    let input = " ".repeat(5);
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // act
    buffer.skip_spaces(2).unwrap();

    // assert
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"  ");
    assert_eq!(buffer.n_shifted_out, 3);
}

#[test]
fn sanity_test_shift() {
    let input = "abcd12345";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 10];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    buffer.shift_buffer(3, 7);

    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc45");
    assert_eq!(buffer.n_shifted_out, 4);
}

#[test]
fn test_noop_shift_at_pos0() {
    let input = "abc";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(0, 0);

    assert_eq!(buffer.n_bytes, 3);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_noop_shift_at_pos1() {
    let input = "abc";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(1, 1);

    assert_eq!(buffer.n_bytes, 3);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_noop_shift_at_end_minus1() {
    let input = "abc";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(2, 2);

    assert_eq!(buffer.n_bytes, 3);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_noop_shift_at_end() {
    let input = "abc";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(3, 3);

    assert_eq!(buffer.n_bytes, 3);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_shift_pos1_to_pos0() {
    let input = "abcd12345";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 10];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(0, 1);

    assert_eq!(&buffer.buf[..buffer.n_bytes], b"bcd12345");
    assert_eq!(buffer.n_shifted_out, 1);
}

#[test]
fn test_shift_preend_to_pos0() {
    let input = "abcd12345";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 10];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(0, input.len() - 1);

    assert_eq!(&buffer.buf[..buffer.n_bytes], b"5");
    assert_eq!(buffer.n_shifted_out, input.len() - 1);
}

#[test]
fn test_shift_preend_to_pos1() {
    let input = "abcd12345";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 10];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(1, input.len() - 1);

    assert_eq!(&buffer.buf[..buffer.n_bytes], b"a5");
    assert_eq!(buffer.n_shifted_out, input.len() - 2);
}

#[test]
fn test_shift_end_to_pos0() {
    let input = "abcd12345";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 10];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(0, input.len());

    assert_eq!(&buffer.buf[..buffer.n_bytes], b"");
    assert_eq!(buffer.n_shifted_out, input.len());
}

#[test]
fn test_shift_end_to_pos1() {
    let input = "abcd12345";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 10];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(1, input.len());

    assert_eq!(&buffer.buf[..buffer.n_bytes], b"a");
    assert_eq!(buffer.n_shifted_out, input.len() - 1);
}

#[test]
fn test_shift_postend_to_pos0() {
    let input = "abcd12345";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 10];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(0, input.len() + 1);

    assert_eq!(&buffer.buf[..buffer.n_bytes], b"");
    assert_eq!(buffer.n_shifted_out, input.len());
}

#[test]
fn test_shift_postend_to_pos1() {
    let input = "abcd12345";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 10];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    buffer.shift_buffer(1, input.len() + 1);

    assert_eq!(&buffer.buf[..buffer.n_bytes], b"a");
    assert_eq!(buffer.n_shifted_out, input.len() - 1);
}

#[test]
fn test_shift_position_newlines() {
    let input = "abc\ndef\nghi\njkl";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 32];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    let pos_d = input.find('d').unwrap();
    let pos_i = input.find('i').unwrap();

    assert_eq!(buffer.pos_shifted, LinePosition::new(0, 0));

    buffer.shift_buffer(0, pos_d);
    assert_eq!(buffer.pos_shifted, LinePosition::new(1, 0));

    buffer.shift_buffer(0, pos_i - pos_d);
    assert_eq!(buffer.pos_shifted, LinePosition::new(2, 2));
}

#[test]
fn test_shift_position_no_newlines() {
    let input = "abcdefghijkl";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 32];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();
    let pos_d = input.find('d').unwrap();
    let pos_i = input.find('i').unwrap();

    buffer.shift_buffer(0, pos_d);
    assert_eq!(buffer.pos_shifted, LinePosition::new(0, pos_d));

    buffer.shift_buffer(0, pos_i - pos_d);
    assert_eq!(buffer.pos_shifted, LinePosition::new(0, pos_i));
}

#[test]
fn test_shift_position_multiple_reads() {
    let input = "abc\ndef\nghi\njkl";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 8];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    buffer.shift_buffer(0, 88);
    buffer.read_more().unwrap();
    buffer.shift_buffer(0, input.len() - 8);

    assert_eq!(buffer.pos_shifted, LinePosition::new(3, 3));
}

#[test]
fn test_collect_while_basic() {
    let input = "abc123def";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Collect alphabetic characters from position 0
    let offset = buffer.collect_while(|b| b.is_ascii_alphabetic(), 0, true).unwrap();

    assert_eq!(offset, 3); // Stops at '1'
    assert_eq!(&buffer.buf[..offset], b"abc");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_collect_while_from_non_zero_pos() {
    let input = "abc123def";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Collect digits starting from position 3
    let offset = buffer.collect_while(|b| b.is_ascii_digit(), 3, true).unwrap();

    assert_eq!(offset, 6); // Stops at 'd'
    assert_eq!(&buffer.buf[3..offset], b"123");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_collect_while_until_eof() {
    let input = "abcdef";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Collect all alphabetic characters (should reach EOF)
    let offset = buffer.collect_while(|b| b.is_ascii_alphabetic(), 0, true).unwrap();

    assert_eq!(offset, 6); // EOF reached
    assert_eq!(&buffer.buf[..offset], b"abcdef");
    assert_eq!(buffer.n_bytes, 6);
}

#[test]
fn test_collect_while_with_shift() {
    let input = "XXaaaaaaaaaa123"; // "XX" prefix + 10 'a's + "123"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 6]; // Small buffer to force shifting
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect all 'a's starting from position 2 (after "XX")
    // When buffer fills, shift discards the "XX" prefix (everything before pos 2)
    let offset = buffer.collect_while(|b| b == b'a', 2, true).unwrap();

    assert_eq!(offset, 0); // After shift, rejection is at position 0
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"123");
    assert_eq!(buffer.n_shifted_out, 12); // 2 ('XX') + 10 ('a's) shifted out
}

#[test]
fn test_collect_while_with_shift_from_pos1() {
    let input = "Xaaaaaaaaa123"; // 'X' prefix + 9 'a's + "123"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 5]; // Small buffer to force shifting
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect all 'a's starting from position 1 (after "X")
    // When buffer fills, shift discards the "X" prefix (everything before pos 1)
    let offset = buffer.collect_while(|b| b == b'a', 1, true).unwrap();

    assert_eq!(offset, 0); // After shift, rejection is at position 0
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"123");
    assert_eq!(buffer.n_shifted_out, 10); // 1 ('X') + 9 ('a's) shifted out
}

#[test]
fn test_collect_while_buffer_full_error() {
    let input = "aaaaaaaaaa"; // 10 'a's (all acceptable)
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4]; // Small buffer that will fill up
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Try to collect all 'a's - should fail because buffer is full even after shifting
    let result = buffer.collect_while(|b| b.is_ascii_alphabetic(), 0, true);

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e.error_type, rjiter::error::ErrorType::BufferFull));
    }
}

#[test]
fn test_collect_while_rejection_after_read() {
    let input = "aaaaaaa123"; // 7 'a's + "123"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4]; // Small buffer
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect 'a's - needs to read multiple times before finding '1'
    let offset = buffer.collect_while(|b| b == b'a', 0, true).unwrap();

    assert_eq!(offset, 0); // After shift, '1' is at position 0
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"123");
    assert_eq!(buffer.n_shifted_out, 7);
}

#[test]
fn test_collect_while_immediate_rejection() {
    let input = "123abc";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Try to collect alphabetic from position 0, but first byte is '1'
    let offset = buffer.collect_while(|b| b.is_ascii_alphabetic(), 0, true).unwrap();

    assert_eq!(offset, 0); // Immediate rejection
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_collect_while_empty_buffer() {
    let input = "";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Collect from empty buffer
    let offset = buffer.collect_while(|b| b.is_ascii_alphabetic(), 0, true).unwrap();

    assert_eq!(offset, 0);
    assert_eq!(buffer.n_bytes, 0);
}

#[test]
fn test_collect_while_rejection_in_full_buffer() {
    let input = "aaa1"; // 3 'a's + "1"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4]; // Exactly fits all data
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Buffer is full (4 bytes), but finds rejection without needing to shift
    let offset = buffer.collect_while(|b| b == b'a', 0, true).unwrap();

    assert_eq!(offset, 3); // Stops at '1'
    assert_eq!(&buffer.buf[..3], b"aaa");
    assert_eq!(buffer.n_shifted_out, 0); // No shift needed
}

#[test]
fn test_collect_while_discards_prefix() {
    let input = "PREFIXaaaaa123";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 8];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Collect 'a's starting from position 6 (after "PREFIX")
    // When buffer fills, shift discards "PREFIX" + collected 'a's
    let offset = buffer.collect_while(|b| b == b'a', 6, true).unwrap();

    assert_eq!(offset, 0); // After shift, rejection at position 0
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"123");
    assert_eq!(buffer.n_shifted_out, 11); // 6 ('PREFIX') + 5 ('a's) shifted out
}

#[test]
fn test_collect_while_no_shift_allowed() {
    let input = "aaaa123"; // 4 'a's + "123"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 3]; // Buffer that will fill with 'a's
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Try to collect all 'a's with allow_shift = false - should fail when buffer fills
    let result = buffer.collect_while(|b| b == b'a', 0, false);

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e.error_type, rjiter::error::ErrorType::BufferFull));
    }
}

#[test]
fn test_collect_while_rejection_after_multiple_reads() {
    let input = "aaa1"; // 3 'a's + "1"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 2]; // Very small buffer
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Needs to read multiple times, then finds rejection
    let offset = buffer.collect_while(|b| b == b'a', 0, true).unwrap();

    assert_eq!(offset, 0); // After shifting
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"1");
    assert_eq!(buffer.n_shifted_out, 3);
}
