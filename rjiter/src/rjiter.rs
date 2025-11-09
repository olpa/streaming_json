use embedded_io::{Error as _, Read, Write};

use crate::buffer::Buffer;
use crate::buffer::ChangeFlag;
use crate::error::{can_retry_if_partial, Error as RJiterError, ErrorType, Result as RJiterResult};
use crate::jiter::{
    Jiter, JiterResult, JsonErrorType, JsonValue, LinePosition, NumberAny, NumberInt, Peek,
};

/// Streaming JSON parser, a wrapper around `Jiter`.
pub struct RJiter<'rj, R: Read> {
    jiter: Jiter<'rj>,
    buffer: Buffer<'rj, R>,
}

impl<R: Read> core::fmt::Debug for RJiter<'_, R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "RJiter {{ jiter: {:?}, buffer: {:?} }}",
            self.jiter, self.buffer
        )
    }
}

impl<'rj, R: Read> RJiter<'rj, R> {
    /// Constructs a new `RJiter`.
    ///
    /// # Arguments
    /// - `reader`: The json stream
    /// - `buf`: The working buffer
    pub fn new(reader: &'rj mut R, buf: &'rj mut [u8]) -> Self {
        #[allow(unsafe_code)]
        let buf_alias = unsafe {
            #[allow(mutable_transmutes)]
            #[allow(clippy::transmute_ptr_to_ptr)]
            core::mem::transmute::<&[u8], &'rj mut [u8]>(buf)
        };
        let buffer = Buffer::new(reader, buf_alias);
        // `0 <= buffer.n_bytes <= buf.len()` by the `Buffer` contract
        #[allow(clippy::indexing_slicing)]
        let jiter = Jiter::new(&buf[..buffer.n_bytes]);

        RJiter { jiter, buffer }
    }

    fn create_new_jiter(&mut self) {
        // `0 <= buffer.n_bytes <= buf.len()` by the `Buffer` contract
        #[allow(clippy::indexing_slicing)]
        let jiter_buffer_2 = &self.buffer.buf[..self.buffer.n_bytes];
        #[allow(unsafe_code)]
        let jiter_buffer = unsafe { core::mem::transmute::<&[u8], &'rj [u8]>(jiter_buffer_2) };
        self.jiter = Jiter::new(jiter_buffer);
    }

    //  ------------------------------------------------------------
    // Jiter wrappers
    //

    /// See `Jiter::peek`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn peek(&mut self) -> RJiterResult<Peek> {
        self.loop_until_success(jiter::Jiter::peek, None, false)
    }

    /// See `Jiter::known_array`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_array(&mut self) -> RJiterResult<Option<Peek>> {
        self.loop_until_success(jiter::Jiter::known_array, Some(b'['), false)
    }

    /// See `Jiter::known_bool`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_bool(&mut self, peek: Peek) -> RJiterResult<bool> {
        self.loop_until_success(|j| j.known_bool(peek), None, false)
    }

    /// See `Jiter::known_bytes`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_bytes(&mut self) -> RJiterResult<&[u8]> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<&[u8]>, JiterResult<&'rj [u8]>>(j.known_bytes())
        };
        self.loop_until_success(f, None, false)
    }

    /// See `Jiter::known_float`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_float(&mut self, peek: Peek) -> RJiterResult<f64> {
        self.loop_until_success(|j| j.known_float(peek), None, true)
    }

    /// See `Jiter::known_int`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_int(&mut self, peek: Peek) -> RJiterResult<NumberInt> {
        self.loop_until_success(|j| j.known_int(peek), None, true)
    }

    /// See `Jiter::known_null`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_null(&mut self) -> RJiterResult<()> {
        self.loop_until_success(jiter::Jiter::known_null, None, false)
    }

    /// See `Jiter::known_number`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_number(&mut self, peek: Peek) -> RJiterResult<NumberAny> {
        self.loop_until_success(|j| j.known_number(peek), None, true)
    }

    /// See `Jiter::known_object`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_object(&mut self) -> RJiterResult<Option<&str>> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<Option<&str>>, JiterResult<Option<&'rj str>>>(
                j.known_object(),
            )
        };
        self.loop_until_success(f, Some(b'{'), false)
    }

    /// See `Jiter::known_skip`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_skip(&mut self, peek: Peek) -> RJiterResult<()> {
        self.loop_until_success(|j| j.known_skip(peek), None, true)
    }

    /// See `Jiter::known_str`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_str(&mut self) -> RJiterResult<&str> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<&str>, JiterResult<&'rj str>>(j.known_str())
        };
        self.loop_until_success(f, None, false)
    }

    /// See `Jiter::known_value`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_value(&mut self, peek: Peek) -> RJiterResult<JsonValue<'rj>> {
        self.loop_until_success(|j| j.known_value(peek), None, true)
    }

    /// See `Jiter::known_value_owned`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn known_value_owned(&mut self, peek: Peek) -> RJiterResult<JsonValue<'static>> {
        self.loop_until_success(|j| j.known_value_owned(peek), None, true)
    }

    /// See `Jiter::next_array`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_array(&mut self) -> RJiterResult<Option<Peek>> {
        self.loop_until_success(jiter::Jiter::next_array, Some(b'['), false)
    }

    /// See `Jiter::array_step`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn array_step(&mut self) -> RJiterResult<Option<Peek>> {
        self.loop_until_success(jiter::Jiter::array_step, Some(b','), false)
    }

    /// See `Jiter::next_bool`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_bool(&mut self) -> RJiterResult<bool> {
        self.loop_until_success(jiter::Jiter::next_bool, None, false)
    }

    /// See `Jiter::next_bytes`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_bytes(&mut self) -> RJiterResult<&[u8]> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<&[u8]>, JiterResult<&'rj [u8]>>(j.next_bytes())
        };
        self.loop_until_success(f, None, false)
    }

    /// See `Jiter::next_float`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_float(&mut self) -> RJiterResult<f64> {
        self.loop_until_success(jiter::Jiter::next_float, None, true)
    }

    /// See `Jiter::next_int`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_int(&mut self) -> RJiterResult<NumberInt> {
        self.loop_until_success(jiter::Jiter::next_int, None, true)
    }

    /// See `Jiter::next_key`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_key(&mut self) -> RJiterResult<Option<&str>> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<Option<&str>>, JiterResult<Option<&'rj str>>>(
                j.next_key(),
            )
        };
        self.loop_until_success(f, Some(b','), false)
    }

    /// See `Jiter::next_key_bytes`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_key_bytes(&mut self) -> RJiterResult<Option<&[u8]>> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<Option<&[u8]>>, JiterResult<Option<&'rj [u8]>>>(
                j.next_key_bytes(),
            )
        };
        self.loop_until_success(f, Some(b','), false)
    }

    /// See `Jiter::next_null`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_null(&mut self) -> RJiterResult<()> {
        self.loop_until_success(jiter::Jiter::next_null, None, false)
    }

    /// See `Jiter::next_number`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_number(&mut self) -> RJiterResult<NumberAny> {
        self.loop_until_success(jiter::Jiter::next_number, None, true)
    }

    /// See `Jiter::next_number_bytes`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_number_bytes(&mut self) -> RJiterResult<&[u8]> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<&[u8]>, JiterResult<&'rj [u8]>>(
                j.next_number_bytes(),
            )
        };
        self.loop_until_success(f, None, true)
    }

    /// See `Jiter::next_object`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_object(&mut self) -> RJiterResult<Option<&str>> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<Option<&str>>, JiterResult<Option<&'rj str>>>(
                j.next_object(),
            )
        };
        self.loop_until_success(f, Some(b'{'), false)
    }

    /// See `Jiter::next_object_bytes`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_object_bytes(&mut self) -> RJiterResult<Option<&[u8]>> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<Option<&[u8]>>, JiterResult<Option<&'rj [u8]>>>(
                j.next_object_bytes(),
            )
        };
        self.loop_until_success(f, Some(b'{'), false)
    }

    /// See `Jiter::next_skip`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_skip(&mut self) -> RJiterResult<()> {
        self.loop_until_success(jiter::Jiter::next_skip, None, true)
    }

    /// See `Jiter::next_str`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_str(&mut self) -> RJiterResult<&str> {
        #[allow(unsafe_code)]
        let f = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<&str>, JiterResult<&'rj str>>(j.next_str())
        };
        self.loop_until_success(f, None, false)
    }

    /// See `Jiter::next_value`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_value(&mut self) -> RJiterResult<JsonValue<'rj>> {
        self.loop_until_success(jiter::Jiter::next_value, None, true)
    }

    /// See `Jiter::next_value_owned`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn next_value_owned(&mut self) -> RJiterResult<JsonValue<'static>> {
        self.loop_until_success(jiter::Jiter::next_value_owned, None, true)
    }

    //  ------------------------------------------------------------
    // The implementation of Jiter wrappers
    //

    fn loop_until_success<T, F>(
        &mut self,
        mut f: F,
        skip_spaces_token: Option<u8>,
        should_eager_consume: bool,
    ) -> RJiterResult<T>
    where
        F: FnMut(&mut Jiter<'rj>) -> JiterResult<T>,
        T: core::fmt::Debug,
    {
        // Error-result makes `false`,
        // Ok-result makes `true`, except if the grandcaller hints (`should_eager_consume`) that
        // end of the buffer can be a false positive (e.g. when parsing a number).
        fn downgrade_ok_if_eof<T>(
            result: &JiterResult<T>,
            should_eager_consume: bool,
            jiter: &Jiter,
            n_bytes: usize,
        ) -> bool {
            if !result.is_ok() {
                return false;
            }
            if !should_eager_consume {
                return true;
            }
            if jiter.current_index() < n_bytes {
                return true;
            }
            false
        }
        let jiter_pos = self.jiter.current_index();

        let result = f(&mut self.jiter);
        let is_ok = downgrade_ok_if_eof(
            &result,
            should_eager_consume,
            &self.jiter,
            self.buffer.n_bytes,
        );
        if is_ok {
            // `result` is always `Ok`
            if let Ok(value) = result {
                return Ok(value);
            }
        }

        self.skip_spaces_feeding(jiter_pos, skip_spaces_token)?;

        loop {
            let result = f(&mut self.jiter);

            if let Err(e) = &result {
                if !can_retry_if_partial(e) {
                    return Err(RJiterError::from_jiter_error(
                        self.current_index(),
                        e.clone(),
                    ));
                }
            }

            if result.is_ok() {
                let really_ok = downgrade_ok_if_eof(
                    &result,
                    should_eager_consume,
                    &self.jiter,
                    self.buffer.n_bytes,
                );
                if really_ok {
                    // `result` is always `Ok`
                    if let Ok(value) = result {
                        return Ok(value);
                    }
                }
            }

            let n_read = self.buffer.read_more();
            match n_read {
                Err(e) => return Err(e),
                Ok(0) => {
                    // EOF is reached in the error state
                    return result
                        .map_err(|e| RJiterError::from_jiter_error(self.current_index(), e));
                }
                Ok(_) => {
                    self.create_new_jiter();
                }
            }
        }
    }

    // If the transparent is found after skipping spaces, skip also spaces after the transparent token
    // If any space is skipped, feed the buffer content to the position 0
    // This function should be called only in a retry handler, otherwise it worsens performance
    fn skip_spaces_feeding(
        &mut self,
        jiter_pos: usize,
        transparent_token: Option<u8>,
    ) -> RJiterResult<()> {
        let to_pos = 0;
        let change_flag = ChangeFlag::new(&self.buffer);

        if jiter_pos > to_pos {
            self.buffer.shift_buffer(to_pos, jiter_pos);
        }
        self.buffer.skip_spaces(to_pos)?;
        if let Some(transparent_token) = transparent_token {
            if to_pos >= self.buffer.n_bytes {
                self.buffer.read_more()?;
            }
            // `0 <= to_pos` (usize), `to_pos < buffer.n_bytes` (if check), `n_bytes <= buf.len()` by the `Buffer` contract
            #[allow(clippy::indexing_slicing)]
            if to_pos < self.buffer.n_bytes && self.buffer.buf[to_pos] == transparent_token {
                self.buffer.skip_spaces(to_pos + 1)?;
            }
        }

        if change_flag.is_changed(&self.buffer) {
            self.create_new_jiter();
        }
        Ok(())
    }

    /// See `Jiter::finish`
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn finish(&mut self) -> RJiterResult<()> {
        loop {
            let finish_in_this_buf = self.jiter.finish();
            // Error here is actually not an error, but a marker that something is found
            // and therefore the jiter is not at the end of the json
            if let Err(e) = finish_in_this_buf {
                return Err(RJiterError::from_jiter_error(self.current_index(), e));
            }
            // The current buffer was all only spaces. Read more.
            if self.jiter.current_index() < self.buffer.buf.len() {
                let n_new_bytes = self.buffer.read_more()?;
                // The end of the json is reached
                if n_new_bytes == 0 {
                    return Ok(());
                }
            }
            self.buffer.shift_buffer(0, self.jiter.current_index());
            self.create_new_jiter();
        }
    }

    //  ------------------------------------------------------------

    /// Get the current index of the parser.
    #[must_use]
    pub fn current_index(&self) -> usize {
        self.jiter.current_index() + self.buffer.n_shifted_out
    }

    /// Get the current `LinePosition` of the parser.
    #[must_use]
    pub fn error_position(&self, index: usize) -> LinePosition {
        let index = index - self.buffer.n_shifted_out;
        let pos = self.jiter.error_position(index);
        LinePosition::new(
            pos.line + self.buffer.pos_shifted.line,
            pos.column + self.buffer.pos_shifted.column,
        )
    }

    //  ------------------------------------------------------------
    // Pass-through long strings and bytes

    //
    // Contract for `write_segment`:
    // - arg 1: `self.buffer.buf`,
    // - arg 2: `1 < segment_end_pos <= self.buffer.n_bytes - 1 <= self.buffer.buf.len() - 1`,
    //          or `1 < segment_end_pos <= self.buffer.n_bytes <= 7`,
    //
    fn handle_long<F, T, W: Write>(
        &mut self,
        parser: F,
        writer: &mut W,
        write_completed: impl Fn(T, usize, &mut W) -> RJiterResult<()>,
        write_segment: impl Fn(&mut [u8], usize, usize, &mut W) -> RJiterResult<()>,
    ) -> RJiterResult<()>
    where
        F: Fn(&mut Jiter<'rj>) -> JiterResult<T>,
        T: core::fmt::Debug,
    {
        loop {
            // Handle simple cases:
            // - The string is completed
            // - The error is not recoverable
            let result = parser(&mut self.jiter);
            if let Ok(value) = result {
                write_completed(value, self.current_index(), writer)?;
                return Ok(());
            }
            // We need `err` in the scope later, therefore we don't use `match` for `result`
            // The Ok-arm is handled above
            #[allow(clippy::unwrap_used)]
            let err = result.unwrap_err();
            if !can_retry_if_partial(&err) {
                return Err(RJiterError::from_jiter_error(self.current_index(), err));
            }

            // Move the string to the beginning of the buffer to avoid corner cases.
            // This code runs at most once, and only on the first loop iteration.
            if self.jiter.current_index() > 0 {
                self.buffer.shift_buffer(0, self.jiter.current_index());
                self.create_new_jiter();
            }

            // Current state: the string is not completed
            // Find out a segment to write

            #[allow(clippy::indexing_slicing)]
            let bs_pos = self.buffer.buf[..self.buffer.n_bytes]
                .iter()
                .position(|&b| b == b'\\');
            let segment_end_pos = match bs_pos {
                // No backslash: the segment is the whole buffer
                // `-1`: To write a segment, the writer needs an extra byte to put the quote character
                None => {
                    if self.buffer.n_bytes == 0 {
                        0
                    } else {
                        self.buffer.n_bytes - 1
                    }
                }
                // Backslash is somewhere in the buffer
                // The segment is the part of the buffer before the backslash
                Some(bs_pos) if bs_pos > 1 => bs_pos,
                // Backslash is the first byte of the buffer
                // The segment is the escape sequence
                Some(bs_pos) => {
                    let buf_len = self.buffer.n_bytes;
                    // [QUOTE, SLASH, CHAR, ....]
                    if buf_len < 3 {
                        bs_pos
                    } else {
                        // `buf_len >= 3` in this branch
                        #[allow(clippy::indexing_slicing)]
                        let after_bs = self.buffer.buf[2];
                        if after_bs != b'u' && after_bs != b'U' {
                            bs_pos + 2
                        } else {
                            // [QUOTE, SLASH, u, HEXDEC, HEXDEC, HEXDEC, HEXDEC, ....]
                            if buf_len < 7 {
                                bs_pos
                            } else {
                                bs_pos + 6
                            }
                        }
                    }
                }
            };

            // Correct the segment end position to not break a unicode code point
            let segment_end_pos = (0..=segment_end_pos)
                .rev()
                .find(
                    #[allow(clippy::indexing_slicing)]
                    |&pos| is_utf8_leading_byte(self.buffer.buf[pos]),
                )
                .unwrap_or(0);

            // Write the segment
            if segment_end_pos > 1 {
                write_segment(
                    self.buffer.buf,
                    segment_end_pos,
                    self.current_index(),
                    writer,
                )?;
                self.buffer.shift_buffer(1, segment_end_pos);
            }

            // Read more and repeat
            let n_new_bytes = self.buffer.read_more()?;
            match n_new_bytes {
                0 => return Err(RJiterError::from_jiter_error(self.current_index(), err)),
                1.. => self.create_new_jiter(),
            }
        }
    }

    /// Write-read-write-read-... until the end of the json string.
    /// The bytes are written as such, without transforming them.
    /// This function is useful to copy a long json string to another json.
    ///
    /// Rjiter should be positioned at the beginning of the json string, on a quote character.
    /// Bounding quotes are not included in the output.
    ///
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn write_long_bytes<W: Write>(&mut self, writer: &mut W) -> RJiterResult<()> {
        fn write_completed<W: Write>(
            bytes: &[u8],
            index: usize,
            writer: &mut W,
        ) -> RJiterResult<()> {
            writer.write_all(bytes).map_err(|e| RJiterError {
                error_type: ErrorType::IoError { kind: e.kind() },
                index,
            })
        }
        fn write_segment<W: Write>(
            bytes: &mut [u8],
            end_pos: usize,
            index: usize,
            writer: &mut W,
        ) -> RJiterResult<()> {
            // See the `write_long` contract. May panic for a small buffer (less than 7 bytes)
            #[allow(clippy::indexing_slicing)]
            writer
                .write_all(&bytes[1..end_pos])
                .map_err(|e| RJiterError {
                    error_type: ErrorType::IoError { kind: e.kind() },
                    index,
                })
        }
        #[allow(unsafe_code)]
        let parser = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<&[u8]>, JiterResult<&'rj [u8]>>(j.known_bytes())
        };
        self.handle_long(parser, writer, write_completed, write_segment)
    }

    /// Write-read-write-read-... until the end of the json string.
    /// Converts the json escapes to the corresponding characters.
    ///
    /// Rjiter should be positioned at the beginning of the json string, on a quote character.
    /// Bounding quotes are not included in the output.
    ///
    /// # Errors
    /// `IoError` or `JiterError`
    pub fn write_long_str<W: Write>(&mut self, writer: &mut W) -> RJiterResult<()> {
        fn write_completed<W: Write>(
            string: &str,
            index: usize,
            writer: &mut W,
        ) -> RJiterResult<()> {
            writer
                .write_all(string.as_bytes())
                .map_err(|e| RJiterError {
                    error_type: ErrorType::IoError { kind: e.kind() },
                    index,
                })
        }
        fn write_segment<W: Write>(
            bytes: &mut [u8],
            end_pos: usize,
            index: usize,
            writer: &mut W,
        ) -> RJiterResult<()> {
            // From the `write_long` contract for a big buffer: `1 < end_pos <= self.buffer.n_bytes - 1`
            // May panic for a small buffer (less than 7 bytes)
            #[allow(clippy::indexing_slicing)]
            let orig_char = bytes[end_pos];
            #[allow(clippy::indexing_slicing)]
            {
                bytes[end_pos] = b'"';
            }
            #[allow(clippy::indexing_slicing)]
            let sub_jiter_buf = &bytes[..=end_pos];
            #[allow(unsafe_code)]
            let sub_jiter_buf = unsafe { core::mem::transmute::<&[u8], &[u8]>(sub_jiter_buf) };
            let mut sub_jiter = Jiter::new(sub_jiter_buf);
            let sub_result = sub_jiter.known_str();
            #[allow(clippy::indexing_slicing)]
            {
                bytes[end_pos] = orig_char;
            }

            match sub_result {
                Ok(string) => writer
                    .write_all(string.as_bytes())
                    .map_err(|e| RJiterError {
                        error_type: ErrorType::IoError { kind: e.kind() },
                        index,
                    }),
                Err(e) => Err(RJiterError::from_jiter_error(index, e)),
            }
        }
        #[allow(unsafe_code)]
        let parser = |j: &mut Jiter<'rj>| unsafe {
            core::mem::transmute::<JiterResult<&str>, JiterResult<&'rj str>>(j.known_str())
        };
        self.handle_long(parser, writer, write_completed, write_segment)
    }

    //  ------------------------------------------------------------
    // Lookahead
    //

    /// Lookahead bytes while a predicate is true, without consuming them.
    /// Returns a slice of the bytes that matched the predicate.
    ///
    /// This is a wrapper around `Buffer::collect_while` that returns a slice
    /// instead of an offset. The bytes are not consumed from the buffer.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns true if the byte should be accepted
    ///
    /// # Errors
    ///
    /// Returns `ErrorType::BufferFull` if the buffer fills up with all accepted bytes.
    /// Also returns errors from the underlying reader.
    pub fn lookahead_while<F>(&mut self, predicate: F) -> RJiterResult<&[u8]>
    where
        F: Fn(u8) -> bool,
    {
        let change_flag = ChangeFlag::new(&self.buffer);

        // jiter.current_index() returns position within its slice view of the buffer
        let start_pos = self.jiter.current_index();
        let n_shifted_before = self.buffer.n_shifted_out;

        // Allow collect_while to shift if needed
        let (mut actual_start, mut end_pos) = self.buffer.collect_while(predicate, start_pos, true)?;

        // If buffer changed, it either shifted in collect_while or just read more data
        if change_flag.is_changed(&self.buffer) {
            // If collect_while didn't shift but we need to (start_pos > 0), shift now
            if n_shifted_before == self.buffer.n_shifted_out && start_pos > 0 {
                self.buffer.shift_buffer(0, start_pos);
                // After manual shift, adjust positions
                end_pos -= start_pos;
                actual_start = 0;
            }
            // Note: if collect_while shifted, actual_start is already 0
            self.create_new_jiter();
        }

        #[allow(clippy::indexing_slicing)]
        let slice = &self.buffer.buf[actual_start..end_pos];

        Ok(slice)
    }

    //  ------------------------------------------------------------
    // Skip token
    //

    /// Skip the token if found, otherwise return an error.
    /// `RJiter` should be positioned at the beginning of the potential token using `peek()` or `finish()`
    ///
    /// # Errors
    /// `IoError` or `RJiterError(ExpectedSomeIdent)`
    pub fn known_skip_token(&mut self, token: &[u8]) -> RJiterResult<()> {
        let change_flag = ChangeFlag::new(&self.buffer);
        let mut pos = self.jiter.current_index();
        let mut err_flag = false;

        // Read enough bytes to have the token
        if pos + token.len() >= self.buffer.n_bytes {
            self.buffer.shift_buffer(0, pos);
            pos = 0;
        }
        while self.buffer.n_bytes < pos + token.len() {
            let n_new_bytes = self.buffer.read_more()?;
            if n_new_bytes == 0 {
                // Not an error for the caller, just a normal end of the json
                // The code should create a new Jiter. Doing so below
                err_flag = true;
                break;
            }
        }

        // Find the token
        let found = if err_flag {
            false
        } else {
            // `pos` is `jiter.current_index()` or `0`
            #[allow(clippy::indexing_slicing)]
            let buf_view = &mut self.buffer.buf[pos..self.buffer.n_bytes];
            buf_view.starts_with(token)
        };

        // Sync the Jiter
        if found {
            self.buffer.shift_buffer(0, pos + token.len());
        }
        if change_flag.is_changed(&self.buffer) {
            self.create_new_jiter();
        }

        // Result
        if found {
            return Ok(());
        }
        Err(RJiterError::from_json_error(
            self.current_index(),
            JsonErrorType::ExpectedSomeIdent,
        ))
    }
}

fn is_utf8_leading_byte(b: u8) -> bool {
    // Linters suggests to use `!(0b1000_0000..0b1100_0000).contains(&b)`,
    // but for me the suggestion looks much less readable
    #[allow(clippy::manual_range_contains)]
    let flag = (b < 0b1000_0000) || (b >= 0b1100_0000);
    flag
}
