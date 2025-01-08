use rjiter::buffer::Buffer;
use std::io::Cursor;

#[test]
fn test_basic_skip_spaces() {
    let spaces = " ".repeat(4);
    let input = format!("{spaces}abc");
    let mut reader = Cursor::new(input.as_bytes());
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // act
    buffer.skip_spaces(0);

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

    // act
    buffer.skip_spaces(2);

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
    buffer.skip_spaces(0);

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
    buffer.skip_spaces(2);

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
    buffer.skip_spaces(0);

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
    buffer.skip_spaces(2);

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

    buffer.shift_buffer(3, 3);

    assert_eq!(buffer.n_bytes, 3);
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc");
    assert_eq!(buffer.n_shifted_out, 0);
}
