# RJiter: Streaming JSON parser for Rust

RJiter is a wrapper for [jiter](https://crates.io/crates/jiter) that allows to process a big JSON having a small buffer. "R" stands for "Reader", which fills the buffer on demand.

API documentation:

- [RJiter](https://docs.rs/rjiter/latest/rjiter/). For most functions`, the documentation redirects to `Jiter`
- [Jiter](https://docs.rs/jiter/latest/jiter/)

## RJiter Example

The example repeats the one of Jiter. The only difference is how RJiter is constructed: To parse JSON, it uses the buffer of size 16 bytes.

```rust
use rjiter::{RJiter, NumberInt, Peek};
use std::io::Cursor;

let json_data = r#"
{
    "name": "John Doe", 
    "age": 43,
    "phones": [
        "+44 1234567",
        "+44 2345678"
    ]
}"#;

// Create RJiter
let mut buffer = [0u8; 16];
let mut reader = Cursor::new(json_data.as_bytes());
let mut rjiter = RJiter::new(&mut reader, &mut buffer);

// The rest is again the same as in Jiter
assert_eq!(rjiter.next_object().unwrap(), Some("name"));
assert_eq!(rjiter.next_str().unwrap(), "John Doe");
assert_eq!(rjiter.next_key().unwrap(), Some("age"));
assert_eq!(rjiter.next_int().unwrap(), NumberInt::Int(43));
assert_eq!(rjiter.next_key().unwrap(), Some("phones"));
assert_eq!(rjiter.next_array().unwrap(), Some(Peek::String));
// we know the next value is a string as we just asserted so
assert_eq!(rjiter.known_str().unwrap(), "+44 1234567");
assert_eq!(rjiter.array_step().unwrap(), Some(Peek::String));
// same again
assert_eq!(rjiter.known_str().unwrap(), "+44 2345678");
// next we'll get `None` from `array_step` as the array is finished
assert_eq!(rjiter.array_step().unwrap(), None);
// and `None` from `next_key` as the object is finished
assert_eq!(rjiter.next_key().unwrap(), None);
// and we check there's nothing else in the input
rjiter.finish().unwrap();
```


## Logic and limitations

First, RJiter calls Jiter. If the result is ok, RJiter returns it. Otherwise, the logic is as follows:

- Skip spaces
- Shift the buffer
- Read, try again, read, try again, and so on till the success or till the error can't be fixed by reading more data

The buffer should be large enough to contain each complete JSON element. In the example above, if the buffer size were 12 bytes, the parsing would fail on the telefone numbers:

```
called `Result::unwrap()` on an `Err` value: Error { error_type: JsonError(EofWhileParsingString), index: 79 }
```

The functions that return pointers to bytes, they point to inside the buffer. You should copy the bytes elsewere before calling RJiter again, otherwise RJiter may shift the buffer and the pointers will become invalid.

## Pass-through long stings

Strings can be longer than the buffer, therefore the default logic doesn't work for them. RJiter provides a workaround: The caller provides a writer and RJiter writes the string to it.

- `write_long_bytes`: Copy bytes as is, without touching escapes. Useful for json-to-json conversion.
- `write_long_str`: Unescape the string during copying. Useful for json-to-text conversion.


