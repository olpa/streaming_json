use std::cell::RefCell;

use rjiter::RJiter;
use scan_json::{scan_json, ActionResult, Matcher, Trigger, TriggerEnd};

#[test]
fn test_scan_json_empty_input() {
    let mut reader = std::io::empty();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<()>> = vec![];
    scan_json(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    );
}

#[test]
fn test_scan_json_top_level_types() {
    let json = r#"null true false 42 3.14 "hello" [] {}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<()>> = vec![];
    scan_json(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    );
}

#[test]
fn test_scan_json_simple_object() {
    let json = r#"{"null": null, "bool": true, "num": 42, "float": 3.14, "str": "hello"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<()>> = vec![];
    scan_json(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    );
}

#[test]
fn test_scan_json_simple_array() {
    let json = r#"[null, true, false, 42, 3.14, "hello"]"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let triggers: Vec<Trigger<()>> = vec![];
    scan_json(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    );
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

    let triggers: Vec<Trigger<()>> = vec![];
    scan_json(
        &triggers,
        &vec![],
        &vec![],
        &RefCell::new(rjiter),
        &RefCell::new(()),
    );
}

#[test]
fn test_skip_sse_tokens() {
    let json = r#"data: {"foo": "bar"} [DONE] "#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let sse_tokens = vec!["data:", "DONE"];
    scan_json(
        &vec![],
        &vec![],
        &sse_tokens,
        &RefCell::new(rjiter),
        &RefCell::new(()),
    );
}

#[test]
fn test_call_begin_dont_touch_value() {
    let json = r#"{"foo": "bar", "baz": "qux"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let state = RefCell::new(false);
    let action = Box::new(|_: &RefCell<RJiter>, state: &RefCell<bool>| {
        *state.borrow_mut() = true;
        ActionResult::Ok
    });
    let triggers = vec![Trigger {
        matcher: Matcher::new("foo".to_string(), None, None, None),
        action,
    }];

    scan_json(&triggers, &vec![], &vec![], &RefCell::new(rjiter), &state);
    assert!(*state.borrow(), "Trigger should have been called for 'foo'");
}

#[test]
fn test_call_begin_consume_value() {
    let json = r#"{"foo": "bar", "baz": "qux"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let state = RefCell::new(false);
    let action = Box::new(|rjiter_cell: &RefCell<RJiter>, state: &RefCell<bool>| {
        let mut rjiter = rjiter_cell.borrow_mut();
        let next = rjiter.next_value();
        next.unwrap();

        *state.borrow_mut() = true;
        ActionResult::OkValueIsConsumed
    });
    let triggers = vec![Trigger {
        matcher: Matcher::new("foo".to_string(), None, None, None),
        action,
    }];

    scan_json(&triggers, &vec![], &vec![], &RefCell::new(rjiter), &state);
    assert!(*state.borrow(), "Trigger should have been called for 'foo'");
}

#[test]
fn test_call_end() {
    let json = r#"{"aa": "bb", "foo": {"bar": "baz"}, "baz": "qux"}"#;
    let mut reader = json.as_bytes();
    let mut buffer = vec![0u8; 16];
    let rjiter = RJiter::new(&mut reader, &mut buffer);

    let state = RefCell::new(false);
    let action = Box::new(|state: &RefCell<bool>| *state.borrow_mut() = true);
    let triggers_end = vec![TriggerEnd {
        matcher: Matcher::new("foo".to_string(), None, None, None),
        action,
    }];

    scan_json(
        &vec![],
        &triggers_end,
        &vec![],
        &RefCell::new(rjiter),
        &state,
    );
    assert!(*state.borrow(), "Trigger should have been called for 'foo'");
}
