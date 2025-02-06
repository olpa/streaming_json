# React to events in a JSON stream

This crate is a work in progress.

## Usage

```rust
use std::cell::RefCell;
use std::io::Write;
use scan_json::scan;
use scan_json::{Name, ParentAndName, BoxedAction, BoxedEndAction, StreamOp, Trigger, rjiter::RJiter};


fn scan_llm_output(json: &str) -> RefCell<Vec<u8>> {
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 32];
    let rjiter_cell = RefCell::new(RJiter::new(&mut reader, &mut buffer));
    let writer_cell = RefCell::new(Vec::new());

    let begin_message: Trigger<BoxedAction<dyn Write>> = Trigger::new(
        Box::new(Name::new("message".to_string())),
        Box::new(|_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
            let result = writer.borrow_mut().write_all(b"(new message)\n");
            match result {
                Ok(_) => StreamOp::None,
                Err(e) => StreamOp::Error(Box::new(e)),
            }
        }),
    );
    let content: Trigger<BoxedAction<dyn Write>> = Trigger::new(
        Box::new(Name::new("content".to_string())),
        Box::new(
            |rjiter_cell: &RefCell<RJiter>, writer_cell: &RefCell<dyn Write>| {
                let mut rjiter = rjiter_cell.borrow_mut();
                let mut writer = writer_cell.borrow_mut();
                let result = rjiter.peek()
                    .and_then(|_| rjiter.write_long_bytes(&mut *writer));
                match result {
                    Ok(_) => StreamOp::ValueIsConsumed,
                    Err(e) => StreamOp::Error(Box::new(e)),
                }
            },
        ),
    );
    let end_message: Trigger<BoxedEndAction<dyn Write>> = Trigger::new(
        Box::new(Name::new("message".to_string())),
        Box::new(|writer: &RefCell<dyn Write>| {
            writer.borrow_mut().write_all(b"\n")?;
            Ok(())
        }),
    );

    scan(
        &vec![begin_message, content],
        &vec![end_message],
        &vec!["data:", "DONE"],
        &rjiter_cell,
        &writer_cell,
    )
    .unwrap();

    writer_cell
}

// ---------------- Sample LLM output

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

// ---------------- Sample LLM output (streaming)
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
