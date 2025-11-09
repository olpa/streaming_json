use core::cmp::min;
use embedded_io::{Error as _, Read};

use crate::error::{Error, ErrorType, Result as RJiterResult};
use crate::jiter::LinePosition;

/// A buffer for reading JSON data.
/// Is a private struct, the "pub" is only for testing.
pub struct Buffer<'buf, R: Read> {
    reader: &'buf mut R,
    /// The working buffer for reading JSON data.
    pub buf: &'buf mut [u8],
    /// Number of valid bytes in the buffer. Contract: `n_bytes <= buf.len()`
    pub n_bytes: usize,
    /// Number of bytes that have been shifted out of the buffer.
    pub n_shifted_out: usize,
    /// Line position correction due to shifting operations.
    pub pos_shifted: LinePosition,
}

impl<'buf, R: Read> Buffer<'buf, R> {
    /// Creates a new buffer with the given reader and buffer.
    #[must_use]
    pub fn new(reader: &'buf mut R, buf: &'buf mut [u8]) -> Self {
        Buffer {
            reader,
            buf,
            n_bytes: 0,
            n_shifted_out: 0,
            pos_shifted: LinePosition::new(0, 0),
        }
    }

    /// Read from the underlying reader into the buffer.
    ///
    /// Returns the number of bytes read.
    ///
    /// # Errors
    ///
    /// From the underlying reader.
    pub fn read_more(&mut self) -> RJiterResult<usize> {
        // The only place where `n_bytes` is increased is this `read_more` function.
        // As long as `read` works correctly, `n_bytes` is less or equal to the buffer size.
        #[allow(clippy::indexing_slicing)]
        let n_new_bytes = self
            .reader
            .read(&mut self.buf[self.n_bytes..])
            .map_err(|e| Error {
                error_type: ErrorType::IoError { kind: e.kind() },
                index: self.n_bytes,
            })?;
        self.n_bytes += n_new_bytes;
        Ok(n_new_bytes)
    }

    /// Shift the buffer to the left, and update the index and line-column position.
    ///
    /// # Arguments
    ///
    /// * `to_pos`: The position to shift to. Usually is 0 or is 1 for strings.
    /// * `from_pos`: The position to shift from. The case of outside the buffer is handled.
    pub fn shift_buffer(&mut self, to_pos: usize, from_pos: usize) {
        let safe_from_pos = min(from_pos, self.n_bytes);
        if to_pos < safe_from_pos {
            // `to_pos>=0` (`usize`), `to_pos < safe_from_pos` (if-branch), `safe_from_pos`<=`n_bytes <= buf.len()` (contract)
            #[allow(clippy::indexing_slicing)]
            for ch in &self.buf[to_pos..safe_from_pos] {
                if *ch == b'\n' {
                    self.pos_shifted.line += 1;
                    self.pos_shifted.column = 0;
                } else {
                    self.pos_shifted.column += 1;
                }
            }
        }

        if from_pos > to_pos && to_pos < self.n_bytes {
            if from_pos < self.n_bytes {
                self.buf.copy_within(from_pos..self.n_bytes, to_pos);
            }
            let n_shifted_out = safe_from_pos - to_pos;
            self.n_bytes -= n_shifted_out;
            self.n_shifted_out += n_shifted_out;
        }
    }

    /// Skip over any ASCII whitespace characters starting at the given position.
    /// Read-shift-read-shift-read-shift... until non-whitespace is found or EOF is reached.
    ///
    /// # Arguments
    ///
    /// * `pos` - The position in the buffer to start skipping from
    ///
    /// # Errors
    ///
    /// From the underlying reader.
    pub fn skip_spaces(&mut self, pos: usize) -> RJiterResult<()> {
        loop {
            match self.collect_while(|b| b.is_ascii_whitespace(), pos, false) {
                Ok((_start_pos, end_of_whitespace)) => {
                    // Found non-whitespace or EOF
                    if end_of_whitespace > pos {
                        self.shift_buffer(pos, end_of_whitespace);
                    }
                    break;
                }
                Err(e) if e.error_type == ErrorType::BufferFull => {
                    // Buffer is full of whitespace, shift and continue
                    self.shift_buffer(pos, self.n_bytes);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Collect bytes while a predicate is true, starting at the given position.
    /// Returns a tuple of (start_position, end_position) where end_position is the offset
    /// of the first rejected byte, or EOF.
    /// If buffer is full with all accepted bytes, it's an error.
    /// The function can optionally shift the buffer once to discard bytes before `start_pos`.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns true if the byte should be accepted
    /// * `start_pos` - The position in the buffer to start collecting from
    /// * `allow_shift` - If true, allows shifting the buffer once when it fills up (discards bytes before `start_pos`)
    ///
    /// # Errors
    ///
    /// Returns `ErrorType::BufferFull` if the buffer fills up with all accepted bytes.
    /// Also returns errors from the underlying reader.
    pub fn collect_while<F>(
        &mut self,
        predicate: F,
        start_pos: usize,
        allow_shift: bool,
    ) -> RJiterResult<(usize, usize)>
    where
        F: Fn(u8) -> bool,
    {
        let mut i = start_pos;
        let mut current_start = start_pos;
        let mut shifted = false;

        loop {
            // Check bytes while predicate is true
            #[allow(clippy::indexing_slicing)]
            while i < self.n_bytes && predicate(self.buf[i]) {
                i += 1;
            }

            if i < self.n_bytes {
                // Found rejected byte
                return Ok((current_start, i));
            }

            // Reached end of buffer, need more data
            // Check if buffer is full and we need to shift before reading
            if self.n_bytes >= self.buf.len() {
                // Buffer is full, need to shift to make space
                if !allow_shift || shifted || start_pos == 0 {
                    // Shifting not allowed, already shifted, or start_pos=0 (nothing to discard) - error!
                    return Err(Error {
                        error_type: ErrorType::BufferFull,
                        index: self.n_shifted_out,
                    });
                }
                // Shift once to make space, discarding everything before start_pos
                // After shift, everything moves left by start_pos positions
                self.shift_buffer(0, start_pos);
                shifted = true;
                i -= start_pos; // Adjust i to account for the shift
                current_start = 0; // After shift, data starts at position 0
            }

            // Try to read more
            let n_new = self.read_more()?;
            if n_new == 0 {
                // EOF reached, all bytes were accepted
                return Ok((current_start, self.n_bytes));
            }
        }
    }

    /// Collect exactly `count` bytes starting at the given position, or until EOF.
    /// Returns a tuple of (start_position, end_position) where end_position is the offset
    /// after the collected bytes (start_pos + actual_collected).
    /// If buffer is too small to hold the requested bytes, it's an error.
    /// The function can optionally shift the buffer once to discard bytes before `start_pos`.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of bytes to collect
    /// * `start_pos` - The position in the buffer to start collecting from
    /// * `allow_shift` - If true, allows shifting the buffer once when it fills up (discards bytes before `start_pos`)
    ///
    /// # Errors
    ///
    /// Returns `ErrorType::BufferFull` if the buffer is too small to hold the requested bytes.
    /// Also returns errors from the underlying reader.
    pub fn collect_count(
        &mut self,
        count: usize,
        start_pos: usize,
        allow_shift: bool,
    ) -> RJiterResult<(usize, usize)> {
        let mut target = start_pos + count;
        let mut current_start = start_pos;
        let mut shifted = false;

        loop {
            if self.n_bytes >= target {
                // We have collected enough bytes
                return Ok((current_start, target));
            }

            // Need more data
            // Check if buffer is full and we need to shift before reading
            if self.n_bytes >= self.buf.len() {
                // Buffer is full, need to shift to make space
                if !allow_shift || shifted || current_start == 0 {
                    // Shifting not allowed, already shifted, or start_pos=0 (nothing to discard) - error!
                    return Err(Error {
                        error_type: ErrorType::BufferFull,
                        index: self.n_shifted_out,
                    });
                }

                // Check if even after shifting, the buffer would be too small
                let available_after_shift = self.buf.len();
                if count > available_after_shift {
                    // Even after shifting, buffer is too small for the requested count
                    return Err(Error {
                        error_type: ErrorType::BufferFull,
                        index: self.n_shifted_out,
                    });
                }

                // Shift once to make space, discarding everything before current_start
                // After shift, everything moves left by current_start positions
                self.shift_buffer(0, current_start);
                shifted = true;
                // Adjust target to account for the shift
                target -= current_start;
                current_start = 0;
            }

            // Try to read more
            let n_new = self.read_more()?;
            if n_new == 0 {
                // EOF reached before collecting all requested bytes
                return Ok((current_start, self.n_bytes));
            }
        }
    }
}

impl<R: Read> core::fmt::Debug for Buffer<'_, R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Buffer {{ n_bytes: {:?}, buf: {:?}, n_shifted_out: {:?}, pos_shifted: {:?} }}",
            self.n_bytes, self.buf, self.n_shifted_out, self.pos_shifted
        )
    }
}

/// A helper struct to check if the buffer has changed and therefore `Jiter` needs to be recreated.
/// Is a private struct, the "pub" is only for testing.
pub(crate) struct ChangeFlag {
    n_shifted: usize,
    n_bytes: usize,
}

impl ChangeFlag {
    #[must_use]
    pub fn new<R: Read>(buf: &Buffer<R>) -> Self {
        ChangeFlag {
            n_shifted: buf.n_shifted_out,
            n_bytes: buf.n_bytes,
        }
    }

    #[must_use]
    pub fn is_changed<R: Read>(&self, buf: &Buffer<R>) -> bool {
        self.n_shifted != buf.n_shifted_out || self.n_bytes != buf.n_bytes
    }
}
