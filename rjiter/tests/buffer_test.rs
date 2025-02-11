use rjiter::buffer::Buffer;
use rjiter::jiter::LinePosition;
use std::io::Cursor;

#[test]
fn test_read_until_full() {
    let input = "abcdef";
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
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
    let mut reader = Cursor::new(input.as_bytes());
    let mut buf = [0u8; 8];
    let mut buffer = Buffer::new(&mut reader, &mut buf);
    buffer.read_more().unwrap();

    buffer.shift_buffer(0, 88);
    buffer.read_more().unwrap();
    buffer.shift_buffer(0, input.len() - 8);

    assert_eq!(buffer.pos_shifted, LinePosition::new(3, 3));
}
