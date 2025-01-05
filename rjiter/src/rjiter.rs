use std::io::Read;
use std::io::Write;

use jiter::{Jiter, JiterResult, JsonValue, NumberAny, NumberInt};

pub type Peek = jiter::Peek;

pub struct RJiter<'rj> {
    jiter: Jiter<'rj>,
    pos_before_call_jiter: usize,
    reader: &'rj mut dyn Read,
    buffer: &'rj mut [u8],
    bytes_in_buffer: usize,
}

impl<'rj> std::fmt::Debug for RJiter<'rj> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "RJiter {{ jiter: {:?}, pos_before_call_jiter: {:?}, buffer: {:?}, bytes_in_buffer: {:?} }}",
            self.jiter, self.pos_before_call_jiter, self.buffer, self.bytes_in_buffer)
    }
}

impl<'rj> RJiter<'rj> {
    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub fn new(reader: &'rj mut dyn Read, buffer: &'rj mut [u8]) -> Self {
        let bytes_in_buffer = reader.read(buffer).unwrap();
        let jiter_buffer = &buffer[..bytes_in_buffer];
        let rjiter_buffer = unsafe {
            #[allow(mutable_transmutes)]
            #[allow(clippy::transmute_ptr_to_ptr)]
            std::mem::transmute::<&[u8], &'rj mut [u8]>(buffer)
        };

        RJiter {
            jiter: Jiter::new(jiter_buffer).with_allow_partial_strings(),
            pos_before_call_jiter: 0,
            reader,
            buffer: rjiter_buffer,
            bytes_in_buffer,
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn peek(&mut self) -> JiterResult<Peek> {
        self.maybe_feed();
        self.jiter.peek()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_array(&mut self) -> JiterResult<Option<Peek>> {
        self.jiter.known_array()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_bool(&mut self, peek: Peek) -> JiterResult<bool> {
        self.jiter.known_bool(peek)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_bytes(&mut self) -> JiterResult<&[u8]> {
        self.jiter.known_bytes()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_float(&mut self, peek: Peek) -> JiterResult<f64> {
        self.jiter.known_float(peek)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_int(&mut self, peek: Peek) -> JiterResult<NumberInt> {
        self.jiter.known_int(peek)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_null(&mut self) -> JiterResult<()> {
        self.jiter.known_null()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_number(&mut self, peek: Peek) -> JiterResult<NumberAny> {
        self.jiter.known_number(peek)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_object(&mut self) -> JiterResult<Option<&str>> {
        self.jiter.known_object()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_skip(&mut self, peek: Peek) -> JiterResult<()> {
        self.jiter.known_skip(peek)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_str(&mut self) -> JiterResult<&str> {
        self.jiter.known_str()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_value(&mut self, peek: Peek) -> JiterResult<JsonValue<'rj>> {
        self.jiter.known_value(peek)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn known_value_owned(&mut self, peek: Peek) -> JiterResult<JsonValue<'static>> {
        self.jiter.known_value_owned(peek)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_array(&mut self) -> JiterResult<Option<Peek>> {
        self.maybe_feed();
        self.jiter.next_array()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn array_step(&mut self) -> JiterResult<Option<Peek>> {
        self.maybe_feed();
        self.jiter.array_step()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_bool(&mut self) -> JiterResult<bool> {
        self.maybe_feed();
        self.jiter.next_bool()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_bytes(&mut self) -> JiterResult<&[u8]> {
        self.maybe_feed();
        self.jiter.next_bytes()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_float(&mut self) -> JiterResult<f64> {
        self.maybe_feed();
        self.jiter.next_float()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_int(&mut self) -> JiterResult<NumberInt> {
        self.maybe_feed();
        self.jiter.next_int()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_key(&mut self) -> JiterResult<Option<&str>> {
        self.maybe_feed();
        self.jiter.next_key()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_key_bytes(&mut self) -> JiterResult<Option<&[u8]>> {
        self.maybe_feed();
        self.jiter.next_key_bytes()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_null(&mut self) -> JiterResult<()> {
        self.maybe_feed();
        self.jiter.next_null()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_number(&mut self) -> JiterResult<NumberAny> {
        self.maybe_feed();
        self.jiter.next_number()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_number_bytes(&mut self) -> JiterResult<&[u8]> {
        self.maybe_feed();
        self.jiter.next_number_bytes()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_object(&mut self) -> JiterResult<Option<&str>> {
        self.maybe_feed();
        self.jiter.next_object()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_object_bytes(&mut self) -> JiterResult<Option<&[u8]>> {
        self.maybe_feed();
        self.jiter.next_object_bytes()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_skip(&mut self) -> JiterResult<()> {
        self.maybe_feed();
        self.jiter.next_skip()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_str(&mut self) -> JiterResult<&str> {
        self.maybe_feed();
        self.jiter.next_str()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_value(&mut self) -> JiterResult<JsonValue<'rj>> {
        self.feed();
        self.jiter.next_value()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn next_value_owned(&mut self) -> JiterResult<JsonValue<'static>> {
        self.feed();
        self.jiter.next_value_owned()
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn finish(&mut self) -> JiterResult<()> {
        self.maybe_feed();
        self.jiter.finish()
    }

    // ----------------

    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    fn write_long(&mut self, is_string: bool, writer: &mut dyn Write) -> JiterResult<()> {
        loop {
            self.on_before_call_jiter();
            let result = if is_string {
                let result = self.jiter.known_str();
                if let Ok(chunk) = result {
                    Ok(chunk.as_bytes())
                } else {
                    Err(result.unwrap_err())
                }
            } else {
                self.jiter.known_bytes()
            };
            if let Ok(bytes) = result {
                writer.write_all(bytes).unwrap();
                if self.jiter.current_index() <= self.bytes_in_buffer {
                    return Ok(());
                }
                self.on_before_call_jiter();
                if !self.feed_inner(true) {
                    return Ok(());
                }
            } else {
                return Err(result.unwrap_err());
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn write_long_bytes(&mut self, writer: &mut dyn Write) -> JiterResult<()> {
        self.write_long(false, writer)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn write_long_str(&mut self, writer: &mut dyn Write) -> JiterResult<()> {
        self.write_long(true, writer)
    }

    fn on_before_call_jiter(&mut self) {
        self.pos_before_call_jiter = self.jiter.current_index();
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn feed(&mut self) -> bool {
        self.on_before_call_jiter();
        self.feed_inner(false)
    }

    fn feed_inner(&mut self, is_partial_string: bool) -> bool {
        let mut pos = self.pos_before_call_jiter;

        //
        // Skip whitespaces
        //
        if !is_partial_string && pos < self.bytes_in_buffer {
            let mut skip_ws_parser = Jiter::new(&self.buffer[pos..self.bytes_in_buffer]);
            let _ = skip_ws_parser.finish();
            pos += skip_ws_parser.current_index();
        }

        //
        // Copy remaining bytes to the beginning of the buffer
        //
        if pos > 0 {
            if pos < self.bytes_in_buffer {
                assert!(
                    !is_partial_string,
                    "Buffer should be completely consumed in partial string case"
                );
                self.buffer.copy_within(pos..self.bytes_in_buffer, 0);
                self.bytes_in_buffer -= pos;
            } else {
                self.bytes_in_buffer = 0;
            }
        }

        //
        // Read new bytes
        //
        let start_index = if is_partial_string {
            1
        } else {
            self.bytes_in_buffer
        };
        let n_new_bytes = self.reader.read(&mut self.buffer[start_index..]).unwrap();
        self.bytes_in_buffer += n_new_bytes;

        if is_partial_string {
            self.buffer[0] = 34;
            self.bytes_in_buffer += 1;
        }

        //
        // Create new Jiter and inform caller if any new bytes were read
        //
        let jiter_buffer_2 = &self.buffer[..self.bytes_in_buffer];
        let jiter_buffer = unsafe { std::mem::transmute::<&[u8], &'rj [u8]>(jiter_buffer_2) };
        self.jiter = Jiter::new(jiter_buffer).with_allow_partial_strings();

        n_new_bytes > 0
    }

    fn maybe_feed(&mut self) {
        if self.jiter.current_index() > self.bytes_in_buffer / 2 {
            self.feed();
        }
    }

    pub fn skip_token(&mut self, token: &[u8]) -> bool {
        self.maybe_feed();

        let buf_view = &mut self.buffer[self.jiter.current_index()..self.bytes_in_buffer];
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
}
