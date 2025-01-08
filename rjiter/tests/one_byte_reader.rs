use std::io::Read;

pub struct OneByteReader<I>
where
    I: Iterator<Item = u8>,
{
    iter: I,
}

impl<I> OneByteReader<I>
where
    I: Iterator<Item = u8>,
{
    pub fn new(iter: I) -> Self {
        OneByteReader { iter }
    }
}

impl<I> Read for OneByteReader<I>
where
    I: Iterator<Item = u8>,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(next_byte) = self.iter.next() {
            buf[0] = next_byte;
            Ok(1)
        } else {
            Ok(0)
        }
    }
}
