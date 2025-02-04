use std::cell::RefCell;
use std::io::Write;

use ::scan_json::action::{BoxedAction, BoxedEndAction, StreamOp, Trigger};
use ::scan_json::matcher::Name;
use ::scan_json::scan;
use rjiter::RJiter;

#[test]
fn test_scan_json_empty_input() {
    let mut reader = std::io::empty();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    )
    .unwrap();
}

#[test]
fn test_scan_json_top_level_types() {
    let json = r#"null true false 42 3.14 "hello" [] {}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    )
    .unwrap();
}

#[test]
fn test_scan_json_simple_object() {
    let json = r#"{"null": null, "bool": true, "num": 42, "float": 3.14, "str": "hello"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    )
    .unwrap();
}

#[test]
fn test_scan_json_simple_array() {
    let json = r#"[null, true, false, 42, 3.14, "hello"]"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    )
    .unwrap();
}

#[test]
fn test_scan_json_nested_complex() {
    let json = r#"{
        "array_of_objects": [
            {"name": "obj1", "values": [1, 2, 3]},
            {"name": "obj2", "nested": {"x": 10, "y": 20}}
        ],
        "object_with_arrays": {
            "nums": [1, 2, [3, 4, [5, 6]], 7],
            "mixed": [
                {"a": 1},
                [true, false],
                {"b": ["hello", "world"]},
                42
            ]
        },
        "deep_nesting": {
            "level1": {
                "level2": [
                    {"level3": {"value": [1, {"final": "deepest"}]}}
                ]
            }
        }
    }"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 64];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    )
    .unwrap();
}

#[test]
fn test_skip_sse_tokens() {
    let json = r#"data: {"foo": "bar"} [DONE] "#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let sse_tokens = vec!["data:", "DONE"];
    scan(
        &vec![],
        &vec![],
        &sse_tokens,
        &RefCell::new(rjiter),
        &RefCell::new(()),
    )
    .unwrap();
}

#[test]
fn test_call_begin_dont_touch_value() {
    let json = r#"{"foo": "bar", "baz": "qux"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let state = RefCell::new(false);
    let matcher = Box::new(Name::new("foo".to_string()));
    let action: BoxedAction<bool> = Box::new(|_: &RefCell<RJiter>, state: &RefCell<bool>| {
        *state.borrow_mut() = true;
        StreamOp::None
    });
    let triggers = vec![Trigger { matcher, action }];

    scan(&triggers, &vec![], &vec![], &RefCell::new(rjiter), &state).unwrap();
    assert!(*state.borrow(), "Trigger should have been called for 'foo'");
}

#[test]
fn test_call_begin_consume_value() {
    let json = r#"{"foo": "bar", "baz": "qux"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let state = RefCell::new(false);
    let matcher = Box::new(Name::new("foo".to_string()));
    let action: BoxedAction<bool> =
        Box::new(|rjiter_cell: &RefCell<RJiter>, state: &RefCell<bool>| {
            let mut rjiter = rjiter_cell.borrow_mut();
            let next = rjiter.next_value();
            next.unwrap();

            *state.borrow_mut() = true;
            StreamOp::ValueIsConsumed
        });
    let triggers = vec![Trigger { matcher, action }];

    scan(&triggers, &vec![], &vec![], &RefCell::new(rjiter), &state).unwrap();
    assert!(*state.borrow(), "Trigger should have been called for 'foo'");
}

#[test]
fn test_call_end() {
    let json = r#"{"aa": "bb", "foo": {"bar": "baz"}, "baz": "qux"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let state = RefCell::new(false);
    let matcher = Box::new(Name::new("foo".to_string()));
    let action: BoxedEndAction<bool> = Box::new(|state: &RefCell<bool>| *state.borrow_mut() = true);
    let triggers_end = vec![Trigger { matcher, action }];

    scan(
        &vec![],
        &triggers_end,
        &vec![],
        &RefCell::new(rjiter),
        &state,
    )
    .unwrap();
    assert!(*state.borrow(), "Trigger should have been called for 'foo'");
}

#[test]
fn max_nesting_array() {
    let json = "[".repeat(25);
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    let result = scan(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    );
    println!("{:?}", result); // FIXME
    let e = result.unwrap_err();
    assert_eq!(
        format!("{e}"),
        "Max nesting exceeded at position 20 with level 20"
    );
}

#[test]
fn max_nesting_object() {
    let json = "{\"a\":".repeat(25);
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    let result = scan(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    );
    let e = result.unwrap_err();
    assert_eq!(
        format!("{e}"),
        "Max nesting exceeded at position 100 with level 20"
    );
}

#[test]
fn several_objects_top_level() {
    let json = r#"{"foo":1}  {"foo":2}  {"foo":3}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let writer_cell = RefCell::new(Vec::new());

    let matcher = Box::new(Name::new("foo".to_string()));
    let action: BoxedAction<dyn Write> =
        Box::new(|_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
            writer.borrow_mut().write_all(b"foo").unwrap();
            StreamOp::None
        });
    let triggers = vec![Trigger { matcher, action }];

    scan(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &writer_cell,
    )
    .unwrap();

    assert_eq!(*writer_cell.borrow(), b"foofoofoo");
}

fn scan_llm_output(json: &str) -> RefCell<Vec<u8>> {
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 32];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let writer_cell = RefCell::new(Vec::new());

    let begin_message: Trigger<BoxedAction<dyn Write>> = Trigger::new(
        Box::new(Name::new("message".to_string())),
        Box::new(|_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
            writer.borrow_mut().write_all(b"(new message)\n").unwrap();
            StreamOp::None
        }),
    );
    let content: Trigger<BoxedAction<dyn Write>> = Trigger::new(
        Box::new(Name::new("content".to_string())),
        Box::new(
            |rjiter_cell: &RefCell<RJiter>, writer_cell: &RefCell<dyn Write>| {
                let mut rjiter = rjiter_cell.borrow_mut();
                let mut writer = writer_cell.borrow_mut();
                rjiter.peek().unwrap();
                rjiter.write_long_bytes(&mut *writer).unwrap();
                StreamOp::ValueIsConsumed
            },
        ),
    );
    let end_message: Trigger<BoxedEndAction<dyn Write>> = Trigger::new(
        Box::new(Name::new("message".to_string())),
        Box::new(|writer: &RefCell<dyn Write>| {
            writer.borrow_mut().write_all(b"\n").unwrap();
        }),
    );

    scan(
        &vec![begin_message, content],
        &vec![end_message],
        &vec!["data:", "DONE"],
        &RefCell::new(rjiter),
        &writer_cell,
    )
    .unwrap();

    writer_cell
}

#[test]
fn scan_basic_llm_output() {
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
    assert_eq!(
        message,
        "(new message)\nHello! How can I assist you today?\n"
    );
}

#[test]
fn scan_streaming_llm_output() {
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
}

#[test]
fn test_json_to_xml() {
    let json_data = r#"
{
    "name": "John Doe", 
    "age": 43,
    "phones": [
        "+44 1234567",
        "+44 2345678"
    ]
}"#;

    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let writer_cell = RefCell::new(Vec::new());

    struct SideEffectMatcher {
        tag_infix: u8,
        writer_cell: &RefCell<dyn Write>,
    }

    impl Matcher for SideEffectMatcher {
        fn matches(&self, name: &str, _context: &[ContextFrame]) -> bool {
            let writer = self.writer_cell.borrow_mut();
            writer.write(b'<').unwrap();
            writer.write(&self.tag_infix).unwrap();
            writer.write(b'>').unwrap();
            true
        }
    }

    let begin_tag: Trigger<BoxedAction<dyn Write>> = Trigger::new(
        Box::new(SideEffectMatcher {
            tag_infix: None,
            writer_cell: &writer_cell,
        }),
        Box::new(|_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
                let mut rjiter = rjiter_cell.borrow_mut();
                let mut writer = writer_cell.borrow_mut();
                let peek = rjiter.peek().unwrap();
                if peek == Peek::String {
                    rjiter.write_long_bytes(&mut *writer).unwrap();
                    StreamOp::ValueIsConsumed
                } else {
                    StreamOp::None
                }
            },
        ),
    );
    let end_tag: Trigger<BoxedAction<dyn Write>> = Trigger::new(
        Box::new(SideEffectMatcher {
            tag_infix: Some(b"/"),
            writer_cell: &writer_cell,
        }),
        Box::new(|_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| { }),
    );

    scan(
        &vec![begin_tag],
        &vec![end_tag],
        &vec![],
        &RefCell::new(rjiter),
        &writer_cell,
    )
    .unwrap();

    let message = String::from_utf8(writer_cell.borrow().to_vec()).unwrap();
    assert_eq!(message, "Hello! How can I assist you today?");
}
