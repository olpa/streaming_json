use std::io::Read;
use std::io::Write;

use crate::buffer::Buffer;
use jiter::{
    Jiter, JiterError, JiterErrorType, JiterResult, JsonErrorType, JsonValue, NumberAny, NumberInt,
};

pub type Peek = jiter::Peek;

pub type RJiterResult<T> = Result<T, RJiterError>;

#[derive(Debug)]
pub enum RJiterError {
    JiterError(JiterError),
    IoError(std::io::Error),
}

impl From<JiterError> for RJiterError {
    fn from(err: JiterError) -> Self {
        RJiterError::JiterError(err)
    }
}

impl From<std::io::Error> for RJiterError {
    fn from(err: std::io::Error) -> Self {
        RJiterError::IoError(err)
    }
}

pub struct RJiter<'rj> {
    jiter: Jiter<'rj>,
    pos_before_call_jiter: usize,
    buffer: Buffer<'rj>,
}

impl<'rj> std::fmt::Debug for RJiter<'rj> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RJiter {{ jiter: {:?}, pos_before_call_jiter: {:?}, buffer: {:?} }}",
            self.jiter, self.pos_before_call_jiter, self.buffer
        )
    }
}

// Copy-paste from jiter/src/error.rs, where it is private
fn allowed_if_partial(error_type: &JsonErrorType) -> bool {
    matches!(
        error_type,
        JsonErrorType::EofWhileParsingList
                | JsonErrorType::EofWhileParsingObject
                | JsonErrorType::EofWhileParsingString
                | JsonErrorType::EofWhileParsingValue
                | JsonErrorType::ExpectedListCommaOrEnd
                | JsonErrorType::ExpectedObjectCommaOrEnd
        )
}

impl<'rj> RJiter<'rj> {
    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub fn new(reader: &'rj mut dyn Read, buf: &'rj mut [u8]) -> Self {
        let buf_alias = unsafe {
            #[allow(mutable_transmutes)]
            #[allow(clippy::transmute_ptr_to_ptr)]
            std::mem::transmute::<&[u8], &'rj mut [u8]>(buf)
        };
        let buffer = Buffer::new(reader, buf_alias);
        let jiter = Jiter::new(&buf[..buffer.n_bytes]);

        RJiter {
            jiter,
            pos_before_call_jiter: 0,
            buffer,
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn peek(&mut self) -> JiterResult<Peek> {
        self.loop_until_success(
            jiter::Jiter::peek,
            None,
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_array(&mut self) -> JiterResult<Option<Peek>> {
        self.loop_until_success(
            jiter::Jiter::known_array,
            Some(b'['),
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_bool(&mut self, peek: Peek) -> JiterResult<bool> {
        self.loop_until_success(
            |j| j.known_bool(peek),
            None,
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_bytes(&mut self) -> JiterResult<&[u8]> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<&[u8]>, JiterResult<&'rj [u8]>>(j.known_bytes())
        };
        self.loop_until_success(f, None, false)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_float(&mut self, peek: Peek) -> JiterResult<f64> {
        self.loop_until_success(
            |j| j.known_float(peek),
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_int(&mut self, peek: Peek) -> JiterResult<NumberInt> {
        self.loop_until_success(
            |j| j.known_int(peek),
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_null(&mut self) -> JiterResult<()> {
        self.loop_until_success(
            jiter::Jiter::known_null,
            None,
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_number(&mut self, peek: Peek) -> JiterResult<NumberAny> {
        self.loop_until_success(
            |j| j.known_number(peek),
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_object(&mut self) -> JiterResult<Option<&str>> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<Option<&str>>, JiterResult<Option<&'rj str>>>(
                j.known_object(),
            )
        };
        self.loop_until_success(
            f,
            Some(b'{'),
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_skip(&mut self, peek: Peek) -> JiterResult<()> {
        self.loop_until_success(
            |j| j.known_skip(peek),
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_str(&mut self) -> JiterResult<&str> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<&str>, JiterResult<&'rj str>>(j.known_str())
        };
        self.loop_until_success(f, None, false)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_value(&mut self, peek: Peek) -> JiterResult<JsonValue<'rj>> {
        self.loop_until_success(
            |j| j.known_value(peek),
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_value_owned(&mut self, peek: Peek) -> JiterResult<JsonValue<'static>> {
        self.loop_until_success(
            |j| j.known_value_owned(peek),
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_array(&mut self) -> JiterResult<Option<Peek>> {
        self.loop_until_success(
            jiter::Jiter::next_array,
            Some(b'['),
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn array_step(&mut self) -> JiterResult<Option<Peek>> {
        self.loop_until_success(
            jiter::Jiter::array_step,
            Some(b','),
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_bool(&mut self) -> JiterResult<bool> {
        self.loop_until_success(
            jiter::Jiter::next_bool,
            None,
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_bytes(&mut self) -> JiterResult<&[u8]> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<&[u8]>, JiterResult<&'rj [u8]>>(j.next_bytes())
        };
        self.loop_until_success(f, None, false)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_float(&mut self) -> JiterResult<f64> {
        self.loop_until_success(
            jiter::Jiter::next_float,
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_int(&mut self) -> JiterResult<NumberInt> {
        self.loop_until_success(
            jiter::Jiter::next_int,
            None,
            true,
        )
    }

    /// See `Jiter::next_key`
    ///
    /// The chunk from the key name to colon (:) should fit to the buffer.
    ///
    /// # Errors
    ///
    /// See `Jiter::next_key`
    pub fn next_key(&mut self) -> JiterResult<Option<&str>> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<Option<&str>>, JiterResult<Option<&'rj str>>>(
                j.next_key(),
            )
        };
        self.loop_until_success(
            f,
            Some(b','),
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_key_bytes(&mut self) -> JiterResult<Option<&[u8]>> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<Option<&[u8]>>, JiterResult<Option<&'rj [u8]>>>(
                j.next_key_bytes(),
            )
        };
        self.loop_until_success(
            f,
            Some(b','),
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_null(&mut self) -> JiterResult<()> {
        self.loop_until_success(
            jiter::Jiter::next_null,
            None,
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_number(&mut self) -> JiterResult<NumberAny> {
        self.loop_until_success(
            jiter::Jiter::next_number,
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_number_bytes(&mut self) -> JiterResult<&[u8]> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<&[u8]>, JiterResult<&'rj [u8]>>(j.next_number_bytes())
        };
        self.loop_until_success(f, None, true)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_object(&mut self) -> JiterResult<Option<&str>> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<Option<&str>>, JiterResult<Option<&'rj str>>>(
                j.next_object(),
            )
        };
        self.loop_until_success(
            f,
            Some(b'{'),
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_object_bytes(&mut self) -> JiterResult<Option<&[u8]>> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<Option<&[u8]>>, JiterResult<Option<&'rj [u8]>>>(
                j.next_object_bytes(),
            )
        };
        self.loop_until_success(
            f,
            Some(b'{'),
            false,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_skip(&mut self) -> JiterResult<()> {
        self.loop_until_success(
            jiter::Jiter::next_skip,
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_str(&mut self) -> JiterResult<&str> {
        let f = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<&str>, JiterResult<&'rj str>>(j.next_str())
        };
        self.loop_until_success(f, None, false)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_value(&mut self) -> JiterResult<JsonValue<'rj>> {
        self.loop_until_success(
            jiter::Jiter::next_value,
            None,
            true,
        )
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_value_owned(&mut self) -> JiterResult<JsonValue<'static>> {
        self.loop_until_success(
            jiter::Jiter::next_value_owned,
            None,
            true,
        )
    }

    // ----------------

    #[allow(clippy::missing_errors_doc)]
    pub fn finish(&mut self) -> JiterResult<()> {
        loop {
            self.jiter.finish()?;
            if self.buffer.read_more() == 0 {
                return Ok(());
            }
            self.buffer.shift_buffer(0, self.jiter.current_index());
            self.create_new_jiter();
        }
    }

    // ----------------

    fn handle_long<F, T>(
        &mut self,
        parser: F,
        writer: &mut dyn Write,
        write_completed: impl Fn(T, &mut dyn Write) -> RJiterResult<()>,
        write_segment: impl Fn(&mut [u8], usize, &mut dyn Write) -> RJiterResult<()>,
    ) -> RJiterResult<()>
    where
        F: Fn(&mut Jiter<'rj>) -> JiterResult<T>,
        T: std::fmt::Debug,
    {
        loop {
            let result = parser(&mut self.jiter);
            if let Ok(value) = result {
                write_completed(value, writer)?;
                return Ok(());
            }
            let error = result.unwrap_err();
            if error.error_type != JiterErrorType::JsonError(JsonErrorType::EofWhileParsingString) {
                return Err(RJiterError::JiterError(error));
            }

            let mut escaping_bs_pos: usize = self.buffer.n_bytes;
            let mut i: usize = 1; // skip the quote character
            while i < self.buffer.n_bytes {
                if self.buffer.buf[i] == b'\\' {
                    escaping_bs_pos = i;
                    i += 1;
                }
                i += 1;
            }

            self.buffer.buf[0] = b'"';

            if escaping_bs_pos > 1 {
                write_segment(self.buffer.buf, escaping_bs_pos, writer)?;
                self.buffer.shift_buffer(1, escaping_bs_pos);
            }

            if self.buffer.read_more() == 0 {
                return Err(RJiterError::JiterError(error));
            }
            self.create_new_jiter();
        }
    }

    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub fn write_long_bytes(&mut self, writer: &mut dyn Write) -> RJiterResult<()> {
        fn write_completed(bytes: &[u8], writer: &mut dyn Write) -> RJiterResult<()> {
            writer.write_all(bytes)?;
            Ok(())
        }
        fn write_segment(
            bytes: &mut [u8],
            escaping_bs_pos: usize,
            writer: &mut dyn Write,
        ) -> RJiterResult<()> {
            writer.write_all(&bytes[1..escaping_bs_pos])?;
            Ok(())
        }
        let parser = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<&[u8]>, JiterResult<&'rj [u8]>>(j.known_bytes())
        };
        self.handle_long(parser, writer, write_completed, write_segment)
    }

    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub fn write_long_str(&mut self, writer: &mut dyn Write) -> RJiterResult<()> {
        fn write_completed(string: &str, writer: &mut dyn Write) -> RJiterResult<()> {
            writer.write_all(string.as_bytes())?;
            Ok(())
        }
        fn write_segment(
            bytes: &mut [u8],
            escaping_bs_pos: usize,
            writer: &mut dyn Write,
        ) -> RJiterResult<()> {
            bytes[escaping_bs_pos] = b'"';
            let sub_jiter_buf = &bytes[..=escaping_bs_pos];
            let sub_jiter_buf = unsafe { std::mem::transmute::<&[u8], &[u8]>(sub_jiter_buf) };
            let mut sub_jiter = Jiter::new(sub_jiter_buf);
            let sub_result = sub_jiter.known_str();
            bytes[escaping_bs_pos] = b'\\';

            match sub_result {
                Ok(string) => {
                    writer.write_all(string.as_bytes())?;
                    Ok(())
                }
                Err(e) => Err(RJiterError::JiterError(e)),
            }
        }
        let parser = |j: &mut Jiter<'rj>| unsafe {
            std::mem::transmute::<JiterResult<&str>, JiterResult<&'rj str>>(j.next_str())
        };
        self.handle_long(parser, writer, write_completed, write_segment)
    }

    fn on_before_call_jiter(&mut self) {
        self.pos_before_call_jiter = self.jiter.current_index();
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn feed(&mut self) -> bool {
        self.on_before_call_jiter();
        self.feed_inner(false)
    }

    // If the transparent is found after skipping spaces, skip also spaces after the transparent token
    // If any space is skipped, feed the buffer content to the position 0
    // This function should be called only in a retry handler, otherwise it worsens performance
    fn skip_spaces_feeding(&mut self, jiter_pos: usize, transparent_token: Option<u8>) -> bool {
        let to_pos = 0;
        let n_shifted_before = self.buffer.n_shifted_out;

        if jiter_pos > to_pos {
            self.buffer.shift_buffer(to_pos, jiter_pos);
        }
        self.buffer.skip_spaces(to_pos);

        if let Some(transparent_token) = transparent_token {
            if to_pos >= self.buffer.n_bytes {
                self.buffer.read_more();
            }
            if to_pos < self.buffer.n_bytes && self.buffer.buf[to_pos] == transparent_token {
                self.buffer.skip_spaces(to_pos + 1);
            }
        }

        let is_shifted = self.buffer.n_shifted_out > n_shifted_before;
        if is_shifted {
            self.create_new_jiter();
        }

        is_shifted
    }

    fn create_new_jiter(&mut self) {
        let jiter_buffer_2 = &self.buffer.buf[..self.buffer.n_bytes];
        let jiter_buffer = unsafe { std::mem::transmute::<&[u8], &'rj [u8]>(jiter_buffer_2) };
        self.jiter = Jiter::new(jiter_buffer);
    }

    fn feed_inner(&mut self, is_partial_string: bool) -> bool {
        let mut pos = self.pos_before_call_jiter;

        //
        // Skip whitespaces
        //
        if !is_partial_string && pos < self.buffer.n_bytes {
            let mut skip_ws_parser = Jiter::new(&self.buffer.buf[pos..self.buffer.n_bytes]);
            let _ = skip_ws_parser.finish();
            pos += skip_ws_parser.current_index();
        }

        //
        // Copy remaining bytes to the beginning of the buffer
        //
        self.buffer.shift_buffer(0, pos);

        //
        // Read new bytes
        //
        let start_index = if is_partial_string {
            1
        } else {
            self.buffer.n_bytes
        };
        let n_new_bytes = self.buffer.read_more_to_pos(start_index);

        if is_partial_string {
            self.buffer.buf[0] = 34; // Quote character
            self.buffer.n_bytes += 1;
        }

        //
        // Create new Jiter and inform caller if any new bytes were read
        //
        self.create_new_jiter();

        n_new_bytes > 0
    }

    fn maybe_feed(&mut self) {
        if self.jiter.current_index() > self.buffer.n_bytes / 2 {
            self.feed();
        }
    }

    pub fn skip_token(&mut self, token: &[u8]) -> bool {
        self.maybe_feed();

        let buf_view = &mut self.buffer.buf[self.jiter.current_index()..self.buffer.n_bytes];
        if !buf_view.starts_with(token) {
            return false;
        }

        for byte in buf_view.iter_mut().take(token.len()) {
            *byte = b' ';
        }
        let _ = self.jiter.finish(); // feed jiter to the next content
        buf_view[..token.len()].copy_from_slice(token);

        true
    }

    fn loop_until_success<T, F>(
        &mut self,
        mut f: F,
        skip_spaces_token: Option<u8>,
        should_eager_consume: bool,
    ) -> JiterResult<T>
    where
        F: FnMut(&mut Jiter<'rj>) -> JiterResult<T>,
        T: std::fmt::Debug,
    {
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
            return result;
        }

        self.skip_spaces_feeding(jiter_pos, skip_spaces_token);

        loop {
            let result = f(&mut self.jiter);

            if result.is_err() {
                let can_retry = if let Err(JiterError { error_type: JiterErrorType::JsonError(error_type), .. }) = &result {
                    allowed_if_partial(error_type)
                } else {
                    false
                };

                if !can_retry {
                    return result;
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
                    return result;
                }
            }

            if self.buffer.read_more() > 0 {
                self.create_new_jiter();
                continue;
            }

            return result;
        }
    }
}
