use std::cell::RefCell;
use std::io::Write;

use ::scan_json::action::{BoxedAction, BoxedEndAction, StreamOp, Trigger};
use ::scan_json::matcher::{Name, ParentAndName, ParentParentAndName};
use ::scan_json::{scan, Options};
use rjiter::{jiter::Peek, RJiter};
use u8pool::U8Pool;

#[test]
fn test_scan_json_empty_input() {
    let mut reader = std::io::empty();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();
}

#[test]
fn test_scan_json_top_level_types() {
    let json = r#"null true false 42 3.14 "hello" [] {}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();
}

#[test]
fn test_scan_json_simple_object() {
    let json = r#"{"null": null, "bool": true, "num": 42, "float": 3.14, "str": "hello"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();
}

#[test]
fn test_scan_json_simple_array() {
    let json = r#"[null, true, false, 42, 3.14, "hello"]"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
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
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();
}

#[test]
fn skip_long_string() {
    let json = format!(r#"{{"foo": "{}", "bar": "baz"}}"#, "a".repeat(100));
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 8];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    scan(
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();
}

#[test]
fn test_skip_sse_tokens() {
    let json = r#"data: {"foo": "bar"} [DONE] "#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let options = Options {
        sse_tokens: vec!["data:".to_string(), "DONE".to_string()],
        stop_early: false,
    };
    scan(
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &options,
    )
    .unwrap();
}

#[test]
fn test_call_begin_dont_touch_value() {
    let json = r#"{"foo": "bar", "baz": "qux"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let state = RefCell::new(false);
    let matcher = Box::new(Name::new("foo".to_string()));
    let action: BoxedAction<bool> = Box::new(|_: &RefCell<RJiter>, state: &RefCell<bool>| {
        *state.borrow_mut() = true;
        StreamOp::None
    });
    let triggers = vec![Trigger { matcher, action }];

    scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &state,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();
    assert!(*state.borrow(), "Trigger should have been called for 'foo'");
}

#[test]
fn test_call_begin_consume_value() {
    let json = r#"{"foo": "bar", "baz": "qux"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

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

    scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &state,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();
    assert!(*state.borrow(), "Trigger should have been called for 'foo'");
}

#[test]
fn test_call_end() {
    let json = r#"{"aa": "bb",
        "foo": {"foo is an object": true, "foo": "nested foo, string"},
        "foo": "string",
        "foo": ["foo is an array"],
        "foo": 42,
        "foo": true,
        "foo": null
    }"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 32];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let state = RefCell::new(0);
    let matcher = Box::new(Name::new("foo".to_string()));
    let action: BoxedEndAction<i32> = Box::new(|state: &RefCell<i32>| {
        *state.borrow_mut() += 1;
        Ok(())
    });
    let triggers_end = vec![Trigger { matcher, action }];

    scan(
        &vec![],
        &triggers_end,
        &RefCell::new(rjiter),
        &state,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();
    assert_eq!(
        *state.borrow(),
        7,
        "Trigger should have been called for end-of-'foo' 7 times"
    );
}

#[test]
fn notify_for_top_level_object() {
    let json = r#"{}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let state = RefCell::new((false, false)); // (begin_called, end_called)

    let begin_action: BoxedAction<(bool, bool)> = Box::new(|_rjiter, state| {
        state.borrow_mut().0 = true;
        StreamOp::None
    });
    let end_action: BoxedEndAction<(bool, bool)> = Box::new(|state| {
        state.borrow_mut().1 = true;
        Ok(())
    });

    let triggers = vec![Trigger {
        matcher: Box::new(ParentAndName::new(
            "#top".to_string(),
            "#object".to_string(),
        )),
        action: begin_action,
    }];
    let triggers_end = vec![Trigger {
        matcher: Box::new(ParentAndName::new(
            "#top".to_string(),
            "#object".to_string(),
        )),
        action: end_action,
    }];

    scan(
        &triggers,
        &triggers_end,
        &RefCell::new(rjiter),
        &state,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();

    let final_state = state.borrow();
    assert!(final_state.0, "Begin trigger should have been called");
    assert!(final_state.1, "End trigger should have been called");
}

#[test]
fn notify_for_object_in_array() {
    let json = r#"[{}, {}, {}]"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let state = RefCell::new((0, 0)); // (begin_called, end_called)

    let begin_action: BoxedAction<(i32, i32)> = Box::new(|_rjiter, state| {
        state.borrow_mut().0 += 1;
        StreamOp::None
    });
    let end_action: BoxedEndAction<(i32, i32)> = Box::new(|state| {
        state.borrow_mut().1 += 1;
        Ok(())
    });

    let triggers = vec![Trigger {
        matcher: Box::new(ParentParentAndName::new(
            "#top".to_string(),
            "#array".to_string(),
            "#object".to_string(),
        )),
        action: begin_action,
    }];
    let triggers_end = vec![Trigger {
        matcher: Box::new(ParentParentAndName::new(
            "#top".to_string(),
            "#array".to_string(),
            "#object".to_string(),
        )),
        action: end_action,
    }];

    scan(
        &triggers,
        &triggers_end,
        &RefCell::new(rjiter),
        &state,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();

    let final_state = state.borrow();
    assert_eq!(
        final_state.0, 3,
        "Begin trigger should have been called 3 times"
    );
    assert_eq!(
        final_state.1, 3,
        "End trigger should have been called 3 times"
    );
}

#[test]
fn notify_for_array() {
    let json = r#"{"items": [1, 2, 3]}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let state = RefCell::new((false, false)); // (begin_called, end_called)

    let begin_action: BoxedAction<(bool, bool)> = Box::new(|_rjiter, state| {
        state.borrow_mut().0 = true;
        StreamOp::None
    });
    let end_action: BoxedEndAction<(bool, bool)> = Box::new(|state| {
        state.borrow_mut().1 = true;
        Ok(())
    });

    let triggers = vec![Trigger {
        matcher: Box::new(ParentAndName::new(
            "items".to_string(),
            "#array".to_string(),
        )),
        action: begin_action,
    }];
    let triggers_end = vec![Trigger {
        matcher: Box::new(ParentAndName::new(
            "items".to_string(),
            "#array".to_string(),
        )),
        action: end_action,
    }];

    scan(
        &triggers,
        &triggers_end,
        &RefCell::new(rjiter),
        &state,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();

    let final_state = state.borrow();
    assert!(final_state.0, "Begin trigger should have been called");
    assert!(final_state.1, "End trigger should have been called");
}

#[test]
fn client_can_consume_array() {
    let json = r#"{"items": [1, 2, 3]}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let writer_cell = RefCell::new(Vec::new());

    let begin_matcher = Box::new(ParentAndName::new(
        "items".to_string(),
        "#array".to_string(),
    ));
    let begin_action: BoxedAction<dyn Write> = Box::new(
        |rjiter_cell: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
            let mut rjiter = rjiter_cell.borrow_mut();
            let mut writer = writer.borrow_mut();
            writer.write_all(b"<array>").unwrap();
            let value = rjiter.next_value().unwrap();
            writer.write_all(format!("{value:?}").as_bytes()).unwrap();
            StreamOp::ValueIsConsumed
        },
    );
    let end_matcher = Box::new(ParentAndName::new(
        "items".to_string(),
        "#array".to_string(),
    ));
    let end_action: BoxedEndAction<dyn Write> = Box::new(|writer: &RefCell<dyn Write>| {
        writer.borrow_mut().write_all(b"</array>").unwrap();
        Ok(())
    });

    let triggers = vec![Trigger {
        matcher: begin_matcher,
        action: begin_action,
    }];
    let triggers_end = vec![Trigger {
        matcher: end_matcher,
        action: end_action,
    }];

    scan(
        &triggers,
        &triggers_end,
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(writer_cell.borrow().to_vec()).unwrap(),
        "<array>Array([Int(1), Int(2), Int(3)])</array>"
    );
}

#[test]
fn several_arrays_top_level() {
    let json = r#"[1,2,3]  [4,5,6]  [7,8,9]"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let writer_cell = RefCell::new(Vec::new());

    let begin_matcher = Box::new(ParentAndName::new("#top".to_string(), "#array".to_string()));
    let begin_action: BoxedAction<dyn Write> =
        Box::new(|_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
            writer.borrow_mut().write_all(b"<array>").unwrap();
            StreamOp::None
        });
    let end_matcher = Box::new(ParentAndName::new("#top".to_string(), "#array".to_string()));
    let end_action: BoxedEndAction<dyn Write> = Box::new(|writer: &RefCell<dyn Write>| {
        writer.borrow_mut().write_all(b"</array>").unwrap();
        Ok(())
    });

    let triggers = vec![Trigger {
        matcher: begin_matcher,
        action: begin_action,
    }];
    let triggers_end = vec![Trigger {
        matcher: end_matcher,
        action: end_action,
    }];

    scan(
        &triggers,
        &triggers_end,
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(writer_cell.borrow().to_vec()).unwrap(),
        "<array></array><array></array><array></array>"
    );
}

#[test]
fn max_nesting_array() {
    let json = "[".repeat(10); // Smaller depth for the test
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 64];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 3).unwrap();

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    let result = scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    );
    let e = result.unwrap_err();
    assert_eq!(
        format!("{e}"),
        "Max nesting exceeded at position 3 with level 3"
    );
}

#[test]
fn max_nesting_object() {
    let json = "{\"a\":".repeat(10);
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 64];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 3).unwrap();

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    let result = scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    );
    let e = result.unwrap_err();
    assert_eq!(
        format!("{e}"),
        "Max nesting exceeded at position 15 with level 3"
    );
}

#[test]
fn error_in_begin_action() {
    let json = r#"{"foo": 123}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 64];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 3).unwrap();

    let matcher = Box::new(Name::new("foo".to_string()));
    let action: BoxedAction<()> = Box::new(|_: &RefCell<RJiter>, _: &RefCell<()>| {
        StreamOp::Error("Test error in begin-action".into())
    });
    let triggers = vec![Trigger { matcher, action }];

    let result = scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    );

    let err = result.unwrap_err();
    assert_eq!(
        format!("{err}"),
        "Action error: Test error in begin-action at position 7"
    );
}

#[test]
fn error_in_end_action() {
    let json = r#"{"foo": 123}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let matcher = Box::new(Name::new("foo".to_string()));
    let end_action: BoxedEndAction<()> =
        Box::new(|_: &RefCell<()>| Err("Test error in end-action".into()));
    let triggers_end = vec![Trigger {
        matcher,
        action: end_action,
    }];

    let result = scan(
        &vec![],
        &triggers_end,
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    );

    let err = result.unwrap_err();
    assert_eq!(
        format!("{err}"),
        "Action error: Test error in end-action at position 11"
    );
}

#[test]
fn several_objects_top_level() {
    let json = r#"{"foo":1}  {"foo":2}  {"foo":3}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let writer_cell = RefCell::new(Vec::new());

    let begin_matcher = Box::new(Name::new("foo".to_string()));
    let begin_action: BoxedAction<dyn Write> =
        Box::new(|_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
            writer.borrow_mut().write_all(b"<foo>").unwrap();
            StreamOp::None
        });
    let end_matcher = Box::new(Name::new("foo".to_string()));
    let end_action: BoxedEndAction<dyn Write> = Box::new(|writer: &RefCell<dyn Write>| {
        writer.borrow_mut().write_all(b"</foo>").unwrap();
        Ok(())
    });

    let triggers = vec![Trigger {
        matcher: begin_matcher,
        action: begin_action,
    }];
    let triggers_end = vec![Trigger {
        matcher: end_matcher,
        action: end_action,
    }];

    scan(
        &triggers,
        &triggers_end,
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();

    assert_eq!(*writer_cell.borrow(), b"<foo></foo><foo></foo><foo></foo>");
}

#[test]
fn match_in_array_context() {
    let json = r#"{"items": [{"name": "first"}, {"name": "second"}]}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let writer_cell = RefCell::new(Vec::new());

    let matcher = Box::new(ParentParentAndName::new(
        "items".to_string(),
        "#array".to_string(),
        "name".to_string(),
    ));
    let action: BoxedAction<dyn Write> = Box::new(
        |rjiter_cell: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
            let mut rjiter = rjiter_cell.borrow_mut();
            let mut writer = writer.borrow_mut();
            let result = rjiter
                .peek()
                .and_then(|_| rjiter.write_long_bytes(&mut *writer));
            match result {
                Ok(_) => StreamOp::ValueIsConsumed,
                Err(e) => StreamOp::Error(Box::new(e)),
            }
        },
    );
    let triggers = vec![Trigger { matcher, action }];

    scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();

    assert_eq!(*writer_cell.borrow(), b"firstsecond");
}

#[test]
fn atoms_on_top_level() {
    let json = r#"null true false 42 3.14 "hello""#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let writer_cell = RefCell::new(Vec::new());

    let begin_matcher = Box::new(ParentAndName::new("#top".to_string(), "#atom".to_string()));
    let begin_action: BoxedAction<dyn Write> = Box::new(
        |rjiter_cell: &RefCell<RJiter>, writer_cell: &RefCell<dyn Write>| {
            let mut rjiter = rjiter_cell.borrow_mut();
            let mut writer = writer_cell.borrow_mut();
            let peek = rjiter.peek().unwrap();
            write!(writer, "(matched {:?})", peek).unwrap();
            StreamOp::None
        },
    );

    let triggers = vec![Trigger {
        matcher: begin_matcher,
        action: begin_action,
    }];

    let result = scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        &Options::new(),
    );
    assert!(result.is_ok());

    let message = String::from_utf8(writer_cell.borrow().to_vec()).unwrap();
    assert_eq!(
        message,
        "(matched Null)(matched True)(matched False)(matched Peek('4'))(matched Peek('3'))(matched String)"
    );
}

#[test]
fn atoms_in_array() {
    let json = r#"[null, true, false, 42, 3.14, "hello"]"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let writer_cell = RefCell::new(Vec::new());

    let begin_matcher = Box::new(ParentAndName::new(
        "#array".to_string(),
        "#atom".to_string(),
    ));
    let begin_action: BoxedAction<dyn Write> = Box::new(
        |rjiter_cell: &RefCell<RJiter>, writer_cell: &RefCell<dyn Write>| {
            let mut rjiter = rjiter_cell.borrow_mut();
            let mut writer = writer_cell.borrow_mut();
            let peek = rjiter.peek().unwrap();
            write!(writer, "(matched {:?})", peek).unwrap();
            StreamOp::None
        },
    );

    let triggers = vec![Trigger {
        matcher: begin_matcher,
        action: begin_action,
    }];

    let result = scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        &Options::new(),
    );
    assert!(result.is_ok());

    let message = String::from_utf8(writer_cell.borrow().to_vec()).unwrap();
    assert_eq!(
        message,
        "(matched Null)(matched True)(matched False)(matched Peek('4'))(matched Peek('3'))(matched String)"
    );
}

#[test]
fn atoms_in_object() {
    let json = r#"{"a": null, "b": true, "c": false, "d": 42, "e": 3.14, "f": "hello"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let writer_cell = RefCell::new(Vec::new());

    fn handle_atom(rjiter_cell: &RefCell<RJiter>, writer_cell: &RefCell<dyn Write>) -> StreamOp {
        let mut rjiter = rjiter_cell.borrow_mut();
        let mut writer = writer_cell.borrow_mut();
        let peek = rjiter.peek().unwrap();
        write!(writer, "(matched {:?})", peek).unwrap();
        StreamOp::None
    }

    let triggers: Vec<Trigger<BoxedAction<dyn Write>>> = vec!['a', 'b', 'c', 'd', 'e', 'f']
        .into_iter()
        .map(|field| {
            let matcher = Box::new(ParentAndName::new(field.to_string(), "#atom".to_string()));
            let action: BoxedAction<dyn Write> = Box::new(handle_atom);
            Trigger { matcher, action }
        })
        .collect();

    let result = scan(
        &triggers,
        &vec![],
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        &Options::new(),
    );
    assert!(result.is_ok());

    let message = String::from_utf8(writer_cell.borrow().to_vec()).unwrap();
    assert_eq!(
        message,
        "(matched Null)(matched True)(matched False)(matched Peek('4'))(matched Peek('3'))(matched String)"
    );
}

#[test]
fn atoms_stream_op_return_values() {
    let json = r#"true false 42 777"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
    let writer_cell = RefCell::new(Vec::new());

    let begin_matcher = Box::new(Name::new("#atom".to_string()));
    let begin_action: BoxedAction<dyn Write> = Box::new(
        |rjiter_cell: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
            let mut rjiter = rjiter_cell.borrow_mut();
            let mut writer = writer.borrow_mut();
            let peeked = rjiter.peek().unwrap();

            match peeked {
                Peek::True => {
                    rjiter.next_value().unwrap();
                    writer.write_all(b"consumed,").unwrap();
                    StreamOp::ValueIsConsumed
                }
                Peek::False => {
                    writer.write_all(b"not consumed,").unwrap();
                    StreamOp::None
                }
                _ => {
                    writer.write_all(b"unexpected,").unwrap();
                    StreamOp::Error("Expected error for the test".into())
                }
            }
        },
    );

    let triggers = vec![Trigger {
        matcher: begin_matcher,
        action: begin_action,
    }];

    let result = scan(
        &triggers,
        &vec![],
        &rjiter_cell,
        &writer_cell,
        &mut scan_stack,
        &Options::new(),
    );

    // Check the output
    let message = String::from_utf8(writer_cell.borrow().to_vec()).unwrap();
    assert_eq!(message, "consumed,not consumed,unexpected,");
    assert!(result.is_err());

    // Check that the next value is still 42
    let num = rjiter_cell.borrow_mut().next_int().unwrap();
    assert!(matches!(num, rjiter::jiter::NumberInt::Int(42)));
}

fn scan_llm_output(json: &str) -> RefCell<Vec<u8>> {
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 32];
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();
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
                let result = rjiter
                    .peek()
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
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        &Options {
            sse_tokens: vec!["data:".to_string(), "DONE".to_string()],
            stop_early: false,
        },
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
fn stop_early() {
    let input = r#"{} [] {"foo": "bar"} [{}, []] 777 true"#;
    let mut reader = input.as_bytes();
    let mut buffer = vec![0u8; 16];

    // Part 1: Process all items when stop_early is false
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    scan(
        &triggers,
        &vec![],
        &rjiter_cell,
        &RefCell::new(()),
        &mut scan_stack,
        &Options {
            sse_tokens: vec![],
            stop_early: false, // `false`
        },
    )
    .unwrap();

    // Verify everything was processed
    rjiter_cell.borrow_mut().finish().unwrap();

    // Part 2: Process only first item when stop_early is true
    let mut reader = input.as_bytes();
    let rjiter = RJiter::new(&mut reader, &mut buffer);
    let rjiter_cell = RefCell::new(rjiter);
    let mut scan_buffer = [0u8; 512];
    let mut scan_stack = U8Pool::new(&mut scan_buffer, 20).unwrap();

    let triggers: Vec<Trigger<BoxedAction<()>>> = vec![];
    for _ in 0..4 {
        scan(
            &triggers,
            &vec![],
            &rjiter_cell,
            &RefCell::new(()),
            &mut scan_stack,
            &Options {
                sse_tokens: vec![],
                stop_early: true, // `true`
            },
        )
        .unwrap();
    }

    // Verify we can still read the next item, which is 777
    let mut rjiter = rjiter_cell.borrow_mut();
    assert_eq!(
        rjiter.next_int().unwrap(),
        rjiter::jiter::NumberInt::Int(777)
    );
}
