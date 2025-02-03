use std::cell::RefCell;

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
    assert_eq!(format!("{:?}", result), "Unbalanced JSON at position: 0");
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
    println!("{:?}", result); // FIXME
    assert_eq!(format!("{:?}", result), "Unbalanced JSON at position: 0");
}
