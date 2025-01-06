use std::io::Cursor;
use rjiter::buffer::Buffer;

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
