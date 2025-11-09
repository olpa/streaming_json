use rjiter::buffer::Buffer;
use rjiter::jiter::LinePosition;

mod one_byte_reader;
use one_byte_reader::OneByteReader;

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
    let (start, offset) = buffer
        .collect_while(|b| b.is_ascii_alphabetic(), 0, true)
        .unwrap();

    assert_eq!((start, offset), (0, 3)); // Stops at '1'
    assert_eq!(&buffer.buf[start..offset], b"abc");
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
    let (start, offset) = buffer
        .collect_while(|b| b.is_ascii_digit(), 3, true)
        .unwrap();

    assert_eq!((start, offset), (3, 6)); // Stops at 'd'
    assert_eq!(&buffer.buf[start..offset], b"123");
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
    let (start, offset) = buffer
        .collect_while(|b| b.is_ascii_alphabetic(), 0, true)
        .unwrap();

    assert_eq!((start, offset), (0, 6)); // EOF reached
    assert_eq!(&buffer.buf[start..offset], b"abcdef");
    assert_eq!(buffer.n_bytes, 6);
}

#[test]
fn test_collect_while_with_shift_from_pos2() {
    let input = "XXaaaa123"; // "XX" prefix + 4 'a's + "123"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 5]; // Small buffer to force shifting
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect all 'a's starting from position 2 (after "XX")
    // When buffer fills, shift discards the "XX" prefix (everything before pos 2)
    let (start, offset) = buffer.collect_while(|b| b == b'a', 2, true).unwrap();

    assert_eq!((start, offset), (0, 4)); // After shift, data starts at 0, rejection is at position 4 (collected 4 'a's)
    assert_eq!(&buffer.buf[..offset], b"aaaa");
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"aaaa1");
    assert_eq!(buffer.n_shifted_out, 2); // Only 2 ('XX') shifted out
}

#[test]
fn test_collect_while_with_shift_from_pos1() {
    let input = "Xaaaa123"; // 'X' prefix + 4 'a's + "123"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 5]; // Small buffer to force shifting
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect all 'a's starting from position 1 (after "X")
    // Buffer fills with "Xaaaa", then shift discards the "X" prefix (everything before pos 1)
    let (start, offset) = buffer.collect_while(|b| b == b'a', 1, true).unwrap();

    assert_eq!((start, offset), (0, 4)); // After shift, data starts at 0, rejection is at position 4 (collected 4 'a's)
    assert_eq!(&buffer.buf[..offset], b"aaaa");
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"aaaa1");
    assert_eq!(buffer.n_shifted_out, 1); // Only 1 ('X') shifted out
}

#[test]
fn test_collect_while_with_shift_from_pos0() {
    let input = "aaaaa123"; // 5 'a's + "123"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 5]; // Buffer too small to reach rejection
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect all 'a's starting from position 0
    // Even with allow_shift=true, there's no prefix to discard, so shift is useless
    // Buffer fills with 'a's but can't reach the rejection byte '1'
    let result = buffer.collect_while(|b| b == b'a', 0, true);

    // Should error because buffer is full and shift from pos 0 doesn't help
    assert_eq!(
        result.unwrap_err().error_type,
        rjiter::error::ErrorType::BufferFull
    );
}

#[test]
fn test_collect_while_buffer_full_error() {
    let input = "XXaaaaaaaaaa"; // "XX" prefix + 10 'a's (all acceptable after prefix)
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4]; // Small buffer that will fill up
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Try to collect all 'a's from pos 2 - shift discards "XX" but buffer still too small
    let result = buffer.collect_while(|b| b.is_ascii_alphabetic(), 2, true);

    assert_eq!(
        result.unwrap_err().error_type,
        rjiter::error::ErrorType::BufferFull
    );
}

#[test]
fn test_collect_while_rejection_after_read() {
    let input = b"aaa1"; // 3 'a's + "1"
    let mut reader = OneByteReader::new(input.iter().copied());
    let mut buf = [0u8; 4]; // Small buffer
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect 'a's - needs to read multiple times (one byte at a time) to find rejection
    let (start, offset) = buffer.collect_while(|b| b == b'a', 0, true).unwrap();

    assert_eq!((start, offset), (0, 3)); // Rejection at position 3
    assert_eq!(&buffer.buf[..offset], b"aaa");
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"aaa1");
    assert_eq!(buffer.n_shifted_out, 0); // No shift needed
}

#[test]
fn test_collect_while_immediate_rejection() {
    let input = "123abc";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Try to collect alphabetic from position 0, but first byte is '1'
    let (start, offset) = buffer
        .collect_while(|b| b.is_ascii_alphabetic(), 0, true)
        .unwrap();

    assert_eq!((start, offset), (0, 0)); // Immediate rejection
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
    let (start, offset) = buffer
        .collect_while(|b| b.is_ascii_alphabetic(), 0, true)
        .unwrap();

    assert_eq!((start, offset), (0, 0));
    assert_eq!(buffer.n_bytes, 0);
}

#[test]
fn test_collect_while_no_shift_allowed() {
    let input = "aaaa123"; // 4 'a's + "123"
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 3]; // Buffer that will fill with 'a's
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Try to collect all 'a's with allow_shift = false - should fail when buffer fills
    let result = buffer.collect_while(|b| b == b'a', 0, false);

    assert_eq!(
        result.unwrap_err().error_type,
        rjiter::error::ErrorType::BufferFull
    );
}

#[test]
fn test_collect_while_rejection_after_multiple_reads() {
    let input = b"Xaaaa111"; // 'X' prefix + 4 'a's + "111"
    let mut reader = OneByteReader::new(input.iter().copied());
    let mut buf = [0u8; 5];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Needs to read multiple times (one byte at a time) before finding rejection
    // When buffer fills with "Xa", shift discards 'X', continues with 'a's
    let (start, offset) = buffer.collect_while(|b| b == b'a', 1, true).unwrap();

    assert_eq!((start, offset), (0, 4)); // After shift, data starts at 0, rejection at position 4
    assert_eq!(&buffer.buf[..offset], b"aaaa");
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"aaaa1");
    assert_eq!(buffer.n_shifted_out, 1);
}

#[test]
fn test_collect_count_basic() {
    let input = "abcdefghi";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Collect exactly 3 bytes from position 0
    let (start, offset) = buffer.collect_count(3, 0, true).unwrap();

    assert_eq!((start, offset), (0, 3));
    assert_eq!(&buffer.buf[start..offset], b"abc");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_collect_count_from_non_zero_pos() {
    let input = "abcdefghi";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Collect 3 bytes starting from position 2
    let (start, offset) = buffer.collect_count(3, 2, true).unwrap();

    assert_eq!((start, offset), (2, 5)); // 2 + 3
    assert_eq!(&buffer.buf[start..offset], b"cde");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_collect_count_until_eof() {
    let input = "abc";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Try to collect 10 bytes but only 3 available
    let (start, offset) = buffer.collect_count(10, 0, true).unwrap();

    assert_eq!((start, offset), (0, 3)); // EOF reached
    assert_eq!(&buffer.buf[start..offset], b"abc");
    assert_eq!(buffer.n_bytes, 3);
}

#[test]
fn test_collect_count_exact_match() {
    let input = "abcdef";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Collect exactly all available bytes
    let (start, offset) = buffer.collect_count(6, 0, true).unwrap();

    assert_eq!((start, offset), (0, 6));
    assert_eq!(&buffer.buf[start..offset], b"abcdef");
}

#[test]
fn test_collect_count_zero_bytes() {
    let input = "abcdef";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Collect 0 bytes
    let (start, offset) = buffer.collect_count(0, 0, true).unwrap();

    assert_eq!((start, offset), (0, 0));
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_collect_count_with_shift_from_pos2() {
    let input = "XXabcdefgh"; // "XX" prefix + 8 bytes
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 5]; // Small buffer to force shifting
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect 4 bytes starting from position 2
    // When buffer fills, shift discards the "XX" prefix
    let (start, offset) = buffer.collect_count(4, 2, true).unwrap();

    assert_eq!((start, offset), (0, 4)); // After shift, collected bytes are at 0-3
    assert_eq!(&buffer.buf[..offset], b"abcd");
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abcde");
    assert_eq!(buffer.n_shifted_out, 2); // "XX" shifted out
}

#[test]
fn test_collect_count_with_shift_from_pos1() {
    let input = "Xabcdefgh"; // "X" prefix + 8 bytes
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 5]; // Small buffer to force shifting
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect 4 bytes starting from position 1
    // Buffer initially reads "Xabcd" (5 bytes), which already contains the 4 bytes needed
    let (start, offset) = buffer.collect_count(4, 1, true).unwrap();

    assert_eq!((start, offset), (1, 5)); // Bytes are at positions 1-4
    assert_eq!(&buffer.buf[start..offset], b"abcd");
    assert_eq!(buffer.n_shifted_out, 0); // No shift needed
}

#[test]
fn test_collect_count_buffer_too_small_from_pos0() {
    let input = "abcdefgh";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 3]; // Buffer too small to hold 5 bytes
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Try to collect 5 bytes from position 0 - buffer too small
    let result = buffer.collect_count(5, 0, true);

    assert_eq!(
        result.unwrap_err().error_type,
        rjiter::error::ErrorType::BufferFull
    );
}

#[test]
fn test_collect_count_buffer_too_small_even_with_shift() {
    let input = "XXabcdefgh"; // "XX" prefix + 8 bytes
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 4]; // Buffer can hold at most 4 bytes total
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Try to collect 5 bytes from position 2
    // Even after shifting "XX", buffer can only hold 4 bytes
    let result = buffer.collect_count(5, 2, true);

    assert_eq!(
        result.unwrap_err().error_type,
        rjiter::error::ErrorType::BufferFull
    );
}

#[test]
fn test_collect_count_no_shift_allowed() {
    let input = "XXabcdefgh"; // "XX" prefix + 8 bytes
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 5]; // Buffer that would need shifting to collect 4 bytes from pos 2
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Try to collect 4 bytes from position 2 with allow_shift = false
    // Buffer fills with "XXabc", needs more data but shifting is not allowed
    let result = buffer.collect_count(4, 2, false);

    assert_eq!(
        result.unwrap_err().error_type,
        rjiter::error::ErrorType::BufferFull
    );
}

#[test]
fn test_collect_count_with_one_byte_reader() {
    let input = b"abcdefgh";
    let mut reader = OneByteReader::new(input.iter().copied());
    let mut buf = [0u8; 10];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect 5 bytes - needs multiple reads (one byte at a time)
    let (start, offset) = buffer.collect_count(5, 0, true).unwrap();

    assert_eq!((start, offset), (0, 5));
    assert_eq!(&buffer.buf[start..offset], b"abcde");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_collect_count_empty_buffer() {
    let input = "";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Try to collect from empty buffer
    let (start, offset) = buffer.collect_count(5, 0, true).unwrap();

    assert_eq!((start, offset), (0, 0)); // EOF immediately
    assert_eq!(buffer.n_bytes, 0);
}

#[test]
fn test_collect_count_eof_from_non_zero_pos() {
    let input = "abc";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Try to collect 10 bytes from position 1, only 2 bytes available
    let (start, offset) = buffer.collect_count(10, 1, true).unwrap();

    assert_eq!((start, offset), (1, 3)); // EOF reached at position 3 (start 1 + collected 2)
    assert_eq!(&buffer.buf[start..offset], b"bc");
}

#[test]
fn test_collect_count_shift_and_multiple_reads() {
    let input = b"XXabcdefgh"; // "XX" prefix + 8 bytes
    let mut reader = OneByteReader::new(input.iter().copied());
    let mut buf = [0u8; 5];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Collect 4 bytes from position 2, needs shift and multiple reads
    let (start, offset) = buffer.collect_count(4, 2, true).unwrap();

    assert_eq!((start, offset), (0, 4)); // After shift
    assert_eq!(&buffer.buf[..offset], b"abcd");
    assert_eq!(buffer.n_shifted_out, 2);
}

//
// skip_n tests
//

#[test]
fn test_skip_n_basic() {
    let input = "abcdefghi";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Skip 3 bytes from position 0 - bytes fit, no shift
    let (new_pos, bytes_skipped) = buffer.skip_n(3, 0).unwrap();

    assert_eq!(new_pos, 3); // New position, no shift
    assert_eq!(bytes_skipped, 3); // Skipped 3 bytes
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abcdefghi"); // Buffer unchanged
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_skip_n_from_non_zero_pos() {
    let input = "abcdefghi";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Skip 3 bytes starting from position 2 - bytes fit, no shift
    let (new_pos, bytes_skipped) = buffer.skip_n(3, 2).unwrap();

    assert_eq!(new_pos, 5); // New position, no shift
    assert_eq!(bytes_skipped, 3);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abcdefghi"); // Buffer unchanged
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_skip_n_until_eof() {
    let input = "abc";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Try to skip 10 bytes but only 3 available - hits EOF
    // With optimization: buffer not full, so no shift before EOF
    let (new_pos, bytes_skipped) = buffer.skip_n(10, 0).unwrap();

    assert_eq!(new_pos, 3); // Position after skipping all 3 bytes
    assert_eq!(bytes_skipped, 3); // Only 3 bytes available
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc"); // Bytes still in buffer
    assert_eq!(buffer.n_shifted_out, 0); // No shift needed
}

#[test]
fn test_skip_n_exact_match() {
    let input = "abcdef";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Skip exactly all available bytes - bytes fit, no shift
    let (new_pos, bytes_skipped) = buffer.skip_n(6, 0).unwrap();

    assert_eq!(new_pos, 6); // New position, no shift
    assert_eq!(bytes_skipped, 6);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abcdef"); // Buffer unchanged
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_skip_n_zero_bytes() {
    let input = "abcdef";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Skip 0 bytes
    let (new_pos, bytes_skipped) = buffer.skip_n(0, 0).unwrap();

    assert_eq!(new_pos, 0);
    assert_eq!(bytes_skipped, 0);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abcdef");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_skip_n_small_buffer_incremental() {
    let input = "abcdefghijklmnopqrstuvwxyz"; // 26 bytes
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 5]; // Small buffer
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Skip 20 bytes - requires multiple iterations (shifts and reads)
    // After shifting 15 bytes (3 iterations of 5), buffer has 5 bytes left "pqrst"
    // Can skip final 5 bytes without shifting
    let (new_pos, bytes_skipped) = buffer.skip_n(20, 0).unwrap();

    assert_eq!(new_pos, 5); // Position in current buffer (shifted 15, skipped last 5)
    assert_eq!(bytes_skipped, 20); // All 20 bytes skipped
    assert_eq!(buffer.n_shifted_out, 15); // Only 15 bytes shifted
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"pqrst"); // Last chunk in buffer
}

#[test]
fn test_skip_n_small_buffer_with_one_byte_reader() {
    let input = b"abcdefghijklmnop"; // 16 bytes
    let mut reader = OneByteReader::new(input.iter().copied());
    let mut buf = [0u8; 4]; // Small buffer
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Skip 10 bytes - OneByteReader reads 1 byte at a time
    // With optimization: buffer fills to 4 bytes before shifting
    // After "abcd" (shift), "efgh" (shift), "ij" remain
    let (new_pos, bytes_skipped) = buffer.skip_n(10, 0).unwrap();

    assert_eq!(new_pos, 2); // Position in current buffer
    assert_eq!(bytes_skipped, 10); // All 10 bytes skipped
    assert_eq!(buffer.n_shifted_out, 8); // 8 bytes shifted (2 shifts of 4)
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"ij"); // Last 2 bytes in buffer
}

#[test]
fn test_skip_n_from_pos_then_skip_more() {
    let input = "abcdefghijklmnop";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // First skip from position 2 - bytes fit, no shift
    let (new_pos1, bytes_skipped1) = buffer.skip_n(3, 2).unwrap();
    assert_eq!(new_pos1, 5); // New position
    assert_eq!(bytes_skipped1, 3);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abcdefghijklmnop");
    assert_eq!(buffer.n_shifted_out, 0);

    // Then skip more from new position - bytes fit, no shift
    let (new_pos2, bytes_skipped2) = buffer.skip_n(4, new_pos1).unwrap();
    assert_eq!(new_pos2, 9); // New position
    assert_eq!(bytes_skipped2, 4);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abcdefghijklmnop");
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_skip_n_empty_buffer() {
    let input = "";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Try to skip from empty buffer
    let (new_pos, bytes_skipped) = buffer.skip_n(5, 0).unwrap();

    assert_eq!(new_pos, 0);
    assert_eq!(bytes_skipped, 0); // No bytes to skip
    assert_eq!(buffer.n_bytes, 0);
    assert_eq!(buffer.n_shifted_out, 0);
}

#[test]
fn test_skip_n_eof_from_non_zero_pos() {
    let input = "abcdef";
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    // Skip from position 4, trying to skip 10 bytes but only 2 available - hits EOF
    // With optimization: buffer not full, so no shift before EOF
    let (new_pos, bytes_skipped) = buffer.skip_n(10, 4).unwrap();

    assert_eq!(new_pos, 6); // Position after skipping 2 bytes from pos 4
    assert_eq!(bytes_skipped, 2); // Only 2 bytes available from position 4
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abcdef"); // Bytes still in buffer
    assert_eq!(buffer.n_shifted_out, 0); // No shift needed
}

#[test]
fn test_skip_n_very_small_buffer_many_bytes() {
    let input = "abcdefghijklmnopqrstuvwxyz0123456789"; // 36 bytes
    let mut reader = input.as_bytes();
    let mut buf = [0u8; 3]; // Very small buffer
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // Skip 30 bytes - requires many iterations with 3-byte buffer
    // After 9 shifts (27 bytes shifted), buffer has bytes 27-29 = "123"
    // Final iteration: available=3 >= remaining(3), returns position 3
    let (new_pos, bytes_skipped) = buffer.skip_n(30, 0).unwrap();

    assert_eq!(new_pos, 3); // Position in current buffer (end of 3-byte buffer)
    assert_eq!(bytes_skipped, 30); // All 30 bytes skipped
    assert_eq!(buffer.n_shifted_out, 27); // 27 bytes shifted
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"123"); // Bytes 27-29 in buffer
}
