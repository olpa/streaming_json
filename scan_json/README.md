# React to elements in a JSON stream

Start processing JSON before the entire document is available.

- [`scan_json` on crates.io](https://crates.io/crates/scan_json)
- [Documentation on docs.rs](https://docs.rs/scan_json/)
- Entry point: [`crate::scan()`]

The goal of `1.2.0` rewrite is to bring zero allocation and `no_std` compatibility. For `1.1.x` branch README, visit the [documentation on docs.rs](https://docs.rs/scan_json/).


## Concepts

The library uses the streaming JSON parser [`RJiter`](https://crates.io/crates/rjiter). While parsing, it maintains context, which is the path of element names from the root to the current nesting level.

The workflow for each key:

- First, call `find_action` and execute if found
- If the key value is an object or array, update the context and parse the next level
- Afterwards, call `find_end_action` and execute if found

An action receives two `RefCell` references as arguments:

- `baton_cell`: A black box for side effects by the action
- `rjiter_cell`: `RJiter` parser object. An action can modify JSON parsing behavior by consuming the current key's value


## Example of an action

`find_action` uses the library helper [`iter_match`] to detect the `content` key and return the `on_content` function.

The action peeks the value and writes it to the output. Because the value is consumed, the action returns the `ValueIsConsumed` flag to `scan` so it can update its internal state.

```rust
use scan_json::{scan, iter_match, BoxedAction, StreamOp, Options};
use scan_json::matcher::StructuralPseudoname;
use scan_json::stack::ContextIter;
use rjiter::RJiter;
use std::cell::RefCell;
use std::io::Write;
use u8pool::U8Pool;

fn on_content(rjiter_cell: &RefCell<RJiter>, writer_cell: &RefCell<dyn Write>) -> StreamOp {
    let mut rjiter = rjiter_cell.borrow_mut();
    let mut writer = writer_cell.borrow_mut();
    let result = rjiter
        .peek()
        .and_then(|_| rjiter.write_long_bytes(&mut *writer));
    match result {
        Ok(_) => StreamOp::ValueIsConsumed,
        Err(e) => StreamOp::Error(Box::new(e)),
    }
}

// Find action function that matches "content" key
let find_action = |structural_pseudoname: StructuralPseudoname, context: ContextIter| -> Option<BoxedAction<dyn Write>> {
    if iter_match(|| ["content".as_bytes()], structural_pseudoname, context) {
        Some(Box::new(on_content))
    } else {
        None
    }
};
```

## Complete example: Identity transformation

The identity transformation copies JSON input to output, retaining the original structure.

The function [`crate::idtransform::idtransform()`] is not just a library function,
but also an example of advanced `scan` use. Read the source code for details.

Additionally, the function [`crate::idtransform::copy_atom()`] can be useful.


## Complete example: converting an LLM stream

Summary:

- Initialize the parser
- Create the black box with a `Vec`, which is used as `dyn Write` in actions
- Create handlers for `message`, `content`, and a handler for the end of `message`
- Combine all together in the `scan` function

The example demonstrates that `scan` can be used to handle LLM streaming output:

- The input consists of several top-level JSON objects not wrapped in an array
- The server-side-events tokens are ignored

```rust
use std::cell::RefCell;
use std::io::Write;
use scan_json::{scan, iter_match, BoxedAction, BoxedEndAction, StreamOp, Options};
use scan_json::matcher::StructuralPseudoname;
use scan_json::stack::ContextIter;
use rjiter::RJiter;
use u8pool::U8Pool;

fn on_begin_message(_: &RefCell<RJiter>, writer: &RefCell<dyn Write>) -> StreamOp {
    let result = writer.borrow_mut().write_all(b"(new message)\n");
    match result {
        Ok(_) => StreamOp::None,
        Err(e) => StreamOp::Error(Box::new(e)),
    }
}

fn on_content(rjiter_cell: &RefCell<RJiter>, writer_cell: &RefCell<dyn Write>) -> StreamOp {
    let mut rjiter = rjiter_cell.borrow_mut();
    let mut writer = writer_cell.borrow_mut();
    let result = rjiter
        .peek()
        .and_then(|_| rjiter.write_long_bytes(&mut *writer));
    match result {
        Ok(_) => StreamOp::ValueIsConsumed,
        Err(e) => StreamOp::Error(Box::new(e)),
    }
}

fn on_end_message(writer: &RefCell<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
    writer.borrow_mut().write_all(b"\n")?;
    Ok(())
}

fn scan_llm_output(json: &str) -> RefCell<Vec<u8>> {
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 32];
    let rjiter_cell = RefCell::new(RJiter::new(&mut reader, &mut buffer));
    let writer_cell = RefCell::new(Vec::new());

    let find_action = |structural_pseudoname: StructuralPseudoname, context: ContextIter| -> Option<BoxedAction<dyn Write>> {
        if iter_match(|| ["content".as_bytes()], structural_pseudoname, context.clone()) {
            Some(Box::new(on_content))
        } else if iter_match(|| ["message".as_bytes()], structural_pseudoname, context.clone()) {
            Some(Box::new(on_begin_message))
        } else {
            None
        }
    };
    let find_end_action = |structural_pseudoname: StructuralPseudoname, context: ContextIter| -> Option<BoxedEndAction<dyn Write>> {
        if iter_match(|| ["message".as_bytes()], structural_pseudoname, context.clone()) {
            Some(Box::new(on_end_message))
        } else {
            None
        }
    };

    // Create working buffer for context stack (512 bytes, up to 20 nesting levels)
    // Based on estimation: 16 bytes per JSON key, plus 8 bytes per frame for state tracking
    let mut working_buffer = [0u8; 512];
    let mut context = U8Pool::new(&mut working_buffer, 20).unwrap();

    scan(
        find_action,
        find_end_action,
        &rjiter_cell,
        &writer_cell,
        &mut context,
        {
            let sse_tokens: &[&[u8]] = &[b"data:", b"DONE"];
            &Options::with_sse_tokens(sse_tokens)
        },
    )
    .unwrap();

    writer_cell
}

// ---------------- Sample LLM output as `scan_llm_output` input

let json = r#"{
  "id": "chatcmpl-Ahpq4nZeP9mESaKsCVdmZdK96IrUH",
  "object": "chat.completion",
  "created": 1735010736,
  "model": "gpt-4o-mini-2024-07-18",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I assist you today?",
        "refusal": null
      },
      "logprobs": null,
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 9,
    "completion_tokens": 10,
    "total_tokens": 19,
    "prompt_tokens_details": {
      "cached_tokens": 0,
      "audio_tokens": 0
    },
    "completion_tokens_details": {
      "reasoning_tokens": 0,
      "audio_tokens": 0,
      "accepted_prediction_tokens": 0,
      "rejected_prediction_tokens": 0
    }
  },
  "system_fingerprint": "fp_0aa8d3e20b"
}"#;

let writer_cell = scan_llm_output(json);
let message = String::from_utf8(writer_cell.borrow().to_vec()).unwrap();
assert_eq!(message, "(new message)\nHello! How can I assist you today?\n");

// ---------------- Another sample of LLM output, the streaming version
let json = r#"
data: {"choices":[{"index":0,"delta":{"role":"assistant","content":"","refusal":null},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{"content":"Hello"},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{"content":"!"},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{"content":" How"},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{"content":" can"},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{"content":" I"},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{"content":" assist"},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{"content":" you"},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{"content":" today"},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{"content":"?"},"logprobs":null,"finish_reason":null}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: {"choices":[{"index":0,"delta":{},"logprobs":null,"finish_reason":"stop"}],"id":"chatcmpl-AgMB1khICnwswjgqIl2X2jr587Nep","object":"chat.completion.chunk","created":1734658387,"model":"gpt-4o-mini-2024-07-18","system_fingerprint":"fp_d02d531b47"}

data: [DONE]
"#;

let writer_cell = scan_llm_output(json);
let message = String::from_utf8(writer_cell.borrow().to_vec()).unwrap();
assert_eq!(message, "Hello! How can I assist you today?");
```


# Colophon

License: MIT

Author: Oleg Parashchenko, olpa@ <https://uucode.com/>

Contact: via email or [Ailets Discord](https://discord.gg/HEBE3gv2)

`scan_json` is a part of the [ailets.org](https://ailets.org) project.
