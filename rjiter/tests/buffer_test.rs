use rjiter::buffer::Buffer;
use std::io::Cursor;

#[test]
fn test_basic_skip_spaces() {
    let input = "    abc".as_bytes();
    let mut reader = Cursor::new(input);
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // act
    buffer.skip_spaces(0);

    // assert
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc");
}

#[test]
fn test_skip_spaces_from_non_zero_pos() {
    let input = "    abc".as_bytes();
    let mut reader = Cursor::new(input);
    let mut buf = [0u8; 16];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // act
    buffer.skip_spaces(2);

    // assert
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"  abc");
}

#[test]
fn test_skip_spaces_with_one_read() {
    let input = "     abc".as_bytes();
    let mut reader = Cursor::new(input);
    let mut buf = [0u8; 4];
    let mut buffer = Buffer::new(&mut reader, &mut buf);

    // act
    buffer.skip_spaces(0);

    // assert
    assert_eq!(&buffer.buf[..buffer.n_bytes], b"abc");
}
