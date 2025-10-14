# `RJiter`: Streaming JSON parser for Rust

`RJiter` allows processing of large JSON files using a small buffer. It is a wrapper for [Jiter](https://crates.io/crates/jiter) and "R" stands for "Reader", which fills the buffer on demand.

API documentation:

- [RJiter](https://docs.rs/rjiter/latest/rjiter/). For most functions, the documentation redirects to `Jiter`
- [Jiter](https://docs.rs/jiter/latest/jiter/)

See also [scan_json](https://crates.io/crates/scan_json) for a callback-based API built on top of `RJiter`.


## Example

The example repeats the one of `Jiter`. The only difference is how `RJiter` is constructed: To parse JSON, it uses the buffer of size 16 bytes.

```rust
use rjiter::jiter::{NumberInt, Peek};
use rjiter::RJiter;

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
let mut reader = json_data.as_bytes();
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

First, `RJiter` calls `Jiter`. If the result is ok, `RJiter` returns it. Otherwise, the logic is as follows:

1. Skip spaces
2. Shift the buffer
3. Read, try again, read, try again, and so on until success or until the error can't be fixed by reading more data

The buffer should be large enough to contain each complete JSON element. In the example above, if the buffer size were 12 bytes, the parsing would fail on the telephone numbers:

```text
called `Result::unwrap()` on an `Err` value: Error { error_type: JsonError(EofWhileParsingString), index: 79 }
```

Functions that return pointers to bytes point inside the buffer. You should copy the bytes elsewhere before calling `RJiter` again; otherwise, `RJiter` may shift the buffer and the pointers will become invalid.


## Pass-through long strings

Strings can be longer than the buffer, therefore the default logic doesn't work for them. `RJiter` provides a workaround: The caller provides a writer and `RJiter` writes the string to it.

- `write_long_bytes`: Copy bytes as is, without touching escapes. Useful for json-to-json conversion.
- `write_long_str`: Unescape the string during copying. Useful for json-to-text conversion.

```rust
use rjiter::RJiter;

let cdata = r#"\"\u4F60\u597d\",\n\\\\\\\\\\\\\\\\\\\\\\\\ how can I help you today?"#;
let input = format!("\"{cdata}\"\"{cdata}\"");

let mut buffer = [0u8; 10];
let mut reader = input.as_bytes();
let mut rjiter = RJiter::new(&mut reader, &mut buffer);

//
// write_long_bytes
//

let mut writer = Vec::new();
let wb = rjiter.write_long_bytes(&mut writer);
wb.unwrap();
assert_eq!(writer, cdata.as_bytes()); // <--- bytes are copied as is

//
// write_long_str
//
let mut writer = Vec::new();
let wb = rjiter.write_long_str(&mut writer);
wb.unwrap();
assert_eq!( // <--- escapes are decoded
    writer,
    r#""你好",
\\\\\\\\\\\\ how can I help you today?"#.as_bytes()
);

let finish = rjiter.finish();
assert!(finish.is_ok());
```


## Skip tokens

For the case when JSON fragments are mixed with known text, `RJiter` provides the function `known_skip_token`.

```rust
use rjiter::{RJiter, Result as RJiterResult};
use rjiter::jiter::Peek;

let json_data = r#"
    event: ping
    data: {"type": "ping"}
"#;

fn peek_skipping_tokens<R: embedded_io::Read>(rjiter: &mut RJiter<R>, tokens: &[&str]) -> RJiterResult<Peek> {
    'outer: loop {
        let peek = rjiter.peek();
        for token in tokens {
            let found = rjiter.known_skip_token(token.as_bytes());
            if found.is_ok() {
                continue 'outer;
            }
        }
        return peek;
    }
}

let mut buffer = [0u8; 10];
let mut reader = json_data.as_bytes();
let mut rjiter = RJiter::new(&mut reader, &mut buffer);

// Skip non-json
let tokens = vec!["data:", "event:", "ping"];
let result = peek_skipping_tokens(&mut rjiter, &tokens);
assert_eq!(result.unwrap(), Peek::Object);

// Continue with json
let key = rjiter.next_object();
assert_eq!(key.unwrap(), Some("type"));
```


## Integration

`RJiter` is compatible with the `no_std` environment:

- It uses **`embedded-io`** instead of `std::io` traits
- **Feature flags**: Enable `std` feature for `Display` trait implementation for errors

Note that while `RJiter` itself is `no_std` compatible, the underlying `Jiter` dependency is not yet `no_std` compatible.


## Colophon

License: MIT

Author: Oleg Parashchenko, olpa@ <https://uucode.com/>

Contact: via email or [Ailets Discord](https://discord.gg/HEBE3gv2)

`RJiter` is a part of the [ailets.org](https://ailets.org) project.
