use embedded_io::Read;

pub struct ChunkReader<'a> {
    data: &'a Vec<u8>,
    position: usize,
    interrupt: u8,
}

impl<'a> ChunkReader<'a> {
    pub fn new(data: &'a Vec<u8>, interrupt: u8) -> Self {
        ChunkReader {
            data,
            position: 0,
            interrupt,
        }
    }
}

impl<'a> embedded_io::ErrorType for ChunkReader<'a> {
    type Error = embedded_io::ErrorKind;
}

impl<'a> Read for ChunkReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.position >= self.data.len() {
            return Ok(0);
        }

        let remaining_data = &self.data[self.position..];
        let chunk_size = remaining_data
            .iter()
            .position(|&b| b == self.interrupt)
            .unwrap_or(remaining_data.len());

        let bytes_to_write = chunk_size.min(buf.len());
        buf[..bytes_to_write].copy_from_slice(&remaining_data[..bytes_to_write]);
        self.position += bytes_to_write;

        // Skip the interrupt character if we found one and haven't reached end of data
        if bytes_to_write == chunk_size && self.position < self.data.len() {
            self.position += 1;
        }

        Ok(bytes_to_write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_reader() {
        let data = vec![1, 2, 3, 0, 4, 5, 0, 6];
        let mut reader = ChunkReader::new(&data, 0);

        let mut buf = [0u8; 10];

        // First chunk: [1, 2, 3]
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], &[1, 2, 3]);

        // Second chunk: [4, 5]
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], &[4, 5]);

        // Third chunk: [6]
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], &[6]);

        // EOF
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 0);
    }
}
