use std::cell::RefCell;
use std::io::Write;

use ::scan_json::matcher::{
    iter_match, BoxedAction, BoxedEndAction, StreamOp, StructuralPseudoname,
};
use ::scan_json::stack::ContextIter;
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

    // find_action that never matches anything
    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    scan(
        find_action,
        find_end_action,
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

    // find_action that never matches anything
    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    scan(
        find_action,
        find_end_action,
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

    // find_action that never matches anything
    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    scan(
        find_action,
        find_end_action,
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

    // find_action that never matches anything
    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    scan(
        find_action,
        find_end_action,
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

    // find_action that never matches anything
    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    scan(
        find_action,
        find_end_action,
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

    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    scan(
        find_action,
        find_end_action,
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

    let sse_tokens: &[&[u8]] = &[b"data:", b"DONE"];
    let options = Options::with_sse_tokens(sse_tokens);

    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    scan(
        find_action,
        find_end_action,
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

    // Action function for when "foo" is matched
    fn set_state_true(_: &RefCell<RJiter>, state: &RefCell<bool>) -> StreamOp {
        *state.borrow_mut() = true;
        StreamOp::None
    }
    // find_action that matches "foo"
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<bool>> {
        if structural_pseudoname == StructuralPseudoname::None {
            if let Some(key) = context.into_iter().next() {
                (key == b"foo").then(|| Box::new(set_state_true) as BoxedAction<bool>)
            } else {
                None
            }
        } else {
            None
        }
    };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<bool>> { None };

    scan(
        find_action,
        find_end_action,
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

    // Action function for when "foo" is matched and value is consumed
    fn consume_foo_value(rjiter_cell: &RefCell<RJiter>, state: &RefCell<bool>) -> StreamOp {
        let mut rjiter = rjiter_cell.borrow_mut();
        let next = rjiter.next_value();
        next.unwrap();

        *state.borrow_mut() = true;
        StreamOp::ValueIsConsumed
    }
    // find_action that matches "foo" and consumes value
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<bool>> {
        if structural_pseudoname == StructuralPseudoname::None {
            if let Some(key) = context.into_iter().next() {
                (key == b"foo").then(|| Box::new(consume_foo_value) as BoxedAction<bool>)
            } else {
                None
            }
        } else {
            None
        }
    };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<bool>> { None };

    scan(
        find_action,
        find_end_action,
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

    // End action function for when "foo" ends
    fn increment_counter(state: &RefCell<i32>) -> Result<(), Box<dyn std::error::Error>> {
        *state.borrow_mut() += 1;
        Ok(())
    }
    // find_action that never matches anything
    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<i32>> { None };
    // find_end_action that matches "foo"
    let find_end_action = |structural_pseudoname: StructuralPseudoname,
                           context: ContextIter|
     -> Option<BoxedEndAction<i32>> {
        if structural_pseudoname == StructuralPseudoname::None {
            if let Some(key) = context.into_iter().next() {
                (key == b"foo").then(|| Box::new(increment_counter) as BoxedEndAction<i32>)
            } else {
                None
            }
        } else {
            None
        }
    };

    scan(
        find_action,
        find_end_action,
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

    // Action functions for #object matching
    fn set_begin_called(_rjiter: &RefCell<RJiter>, state: &RefCell<(bool, bool)>) -> StreamOp {
        state.borrow_mut().0 = true;
        StreamOp::None
    }
    fn set_end_called(state: &RefCell<(bool, bool)>) -> Result<(), Box<dyn std::error::Error>> {
        state.borrow_mut().1 = true;
        Ok(())
    }

    // find_action that matches #object with parent #top
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<(bool, bool)>> {
        iter_match(
            || ["#object".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        )
        .then(|| Box::new(set_begin_called) as BoxedAction<(bool, bool)>)
    };
    // find_end_action that matches #object with parent #top
    let find_end_action = |structural_pseudoname: StructuralPseudoname,
                           context: ContextIter|
     -> Option<BoxedEndAction<(bool, bool)>> {
        iter_match(
            || ["#object".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        )
        .then(|| Box::new(set_end_called) as BoxedEndAction<(bool, bool)>)
    };

    scan(
        find_action,
        find_end_action,
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

    // Action functions for #object in #array matching
    fn increment_begin_count(_rjiter: &RefCell<RJiter>, state: &RefCell<(i32, i32)>) -> StreamOp {
        state.borrow_mut().0 += 1;
        StreamOp::None
    }
    fn increment_end_count(state: &RefCell<(i32, i32)>) -> Result<(), Box<dyn std::error::Error>> {
        state.borrow_mut().1 += 1;
        Ok(())
    }

    // find_action that matches #object with parent #array and grandparent #top
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<(i32, i32)>> {
        iter_match(
            || ["#object".as_bytes(), "#array".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        )
        .then(|| Box::new(increment_begin_count) as BoxedAction<(i32, i32)>)
    };
    // find_end_action that matches #object with parent #array and grandparent #top
    let find_end_action = |structural_pseudoname: StructuralPseudoname,
                           context: ContextIter|
     -> Option<BoxedEndAction<(i32, i32)>> {
        iter_match(
            || ["#object".as_bytes(), "#array".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        )
        .then(|| Box::new(increment_end_count) as BoxedEndAction<(i32, i32)>)
    };

    scan(
        find_action,
        find_end_action,
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

    // Action functions for #array with parent items matching
    fn set_array_begin_called(
        _rjiter: &RefCell<RJiter>,
        state: &RefCell<(bool, bool)>,
    ) -> StreamOp {
        state.borrow_mut().0 = true;
        StreamOp::None
    }
    fn set_array_end_called(
        state: &RefCell<(bool, bool)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        state.borrow_mut().1 = true;
        Ok(())
    }

    // find_action that matches #array with parent items
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<(bool, bool)>> {
        iter_match(
            || ["#array".as_bytes(), "items".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        )
        .then(|| Box::new(set_array_begin_called) as BoxedAction<(bool, bool)>)
    };
    // find_end_action that matches #array with parent items
    let find_end_action = |structural_pseudoname: StructuralPseudoname,
                           context: ContextIter|
     -> Option<BoxedEndAction<(bool, bool)>> {
        iter_match(
            || ["#array".as_bytes(), "items".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        )
        .then(|| Box::new(set_array_end_called) as BoxedEndAction<(bool, bool)>)
    };

    scan(
        find_action,
        find_end_action,
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

    // Action functions for #array with parent items consuming
    fn consume_array_and_write(
        rjiter_cell: &RefCell<RJiter>,
        writer: &RefCell<dyn Write>,
    ) -> StreamOp {
        let mut rjiter = rjiter_cell.borrow_mut();
        let mut writer = writer.borrow_mut();
        writer.write_all(b"Consuming array: ").unwrap();
        let value = rjiter.next_value().unwrap();
        writer.write_all(format!("{value:?}").as_bytes()).unwrap();
        StreamOp::ValueIsConsumed
    }
    fn write_array_end(writer: &RefCell<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        writer.borrow_mut().write_all(b"</array>").unwrap();
        Ok(())
    }

    // find_action that matches #array with parent items
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<dyn Write>> {
        iter_match(
            || ["#array".as_bytes(), "items".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        )
        .then(|| Box::new(consume_array_and_write) as BoxedAction<dyn Write>)
    };
    // find_end_action that matches #array with parent items
    // Will not be called because the array is consumed in the begin action
    let find_end_action = |structural_pseudoname: StructuralPseudoname,
                           context: ContextIter|
     -> Option<BoxedEndAction<dyn Write>> {
        iter_match(
            || ["#array".as_bytes(), "items".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        )
        .then(|| Box::new(write_array_end) as BoxedEndAction<dyn Write>)
    };

    scan(
        find_action,
        find_end_action,
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        &Options::new(),
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(writer_cell.borrow().to_vec()).unwrap(),
        "Consuming array: Array([Int(1), Int(2), Int(3)])"
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

    // find_action that matches #array with parent #top
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<dyn Write>> {
        if iter_match(
            || ["#array".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        ) {
            let action: BoxedAction<dyn Write> =
                Box::new(|_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
                    writer.borrow_mut().write_all(b"<array>").unwrap();
                    StreamOp::None
                });
            Some(action)
        } else {
            None
        }
    };
    // find_end_action that matches #array with parent #top
    let find_end_action = |structural_pseudoname: StructuralPseudoname,
                           context: ContextIter|
     -> Option<BoxedEndAction<dyn Write>> {
        if iter_match(
            || ["#array".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        ) {
            let action: BoxedEndAction<dyn Write> = Box::new(|writer: &RefCell<dyn Write>| {
                writer.borrow_mut().write_all(b"</array>").unwrap();
                Ok(())
            });
            Some(action)
        } else {
            None
        }
    };

    scan(
        find_action,
        find_end_action,
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

    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    let result = scan(
        find_action,
        find_end_action,
        &RefCell::new(rjiter),
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
    );
    let e = result.unwrap_err();
    assert_eq!(
        format!("{e}"),
        "Max nesting exceeded at position 2 with level 3"
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

    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    let result = scan(
        find_action,
        find_end_action,
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

    // find_action that matches "foo" and returns error
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<()>> {
        if iter_match(
            || ["foo".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        ) {
            let action: BoxedAction<()> = Box::new(|_: &RefCell<RJiter>, _: &RefCell<()>| {
                StreamOp::Error("Test error in begin-action".into())
            });
            Some(action)
        } else {
            None
        }
    };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    let result = scan(
        find_action,
        find_end_action,
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

    // find_action that never matches anything
    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    // find_end_action that matches "foo" and returns error
    let find_end_action = |structural_pseudoname: StructuralPseudoname,
                           context: ContextIter|
     -> Option<BoxedEndAction<()>> {
        if iter_match(
            || ["foo".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        ) {
            let action: BoxedEndAction<()> =
                Box::new(|_: &RefCell<()>| Err("Test error in end-action".into()));
            Some(action)
        } else {
            None
        }
    };

    let result = scan(
        find_action,
        find_end_action,
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

    // find_action that matches "foo"
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<dyn Write>> {
        if iter_match(
            || ["foo".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        ) {
            let action: BoxedAction<dyn Write> =
                Box::new(|_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
                    writer.borrow_mut().write_all(b"<foo>").unwrap();
                    StreamOp::None
                });
            Some(action)
        } else {
            None
        }
    };
    // find_end_action that matches "foo"
    let find_end_action = |structural_pseudoname: StructuralPseudoname,
                           context: ContextIter|
     -> Option<BoxedEndAction<dyn Write>> {
        if iter_match(
            || ["foo".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        ) {
            let action: BoxedEndAction<dyn Write> = Box::new(|writer: &RefCell<dyn Write>| {
                writer.borrow_mut().write_all(b"</foo>").unwrap();
                Ok(())
            });
            Some(action)
        } else {
            None
        }
    };

    scan(
        find_action,
        find_end_action,
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

    // find_action that matches name with parent #array and grandparent items
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<dyn Write>> {
        if iter_match(
            || {
                [
                    "name".as_bytes(),
                    "#array".as_bytes(),
                    "items".as_bytes(),
                    "#top".as_bytes(),
                ]
            },
            structural_pseudoname,
            context,
        ) {
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
            Some(action)
        } else {
            None
        }
    };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<dyn Write>> { None };

    scan(
        find_action,
        find_end_action,
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

    // find_action that matches #atom with parent #top
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<dyn Write>> {
        if iter_match(
            || ["#atom".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        ) {
            let action: BoxedAction<dyn Write> = Box::new(
                |rjiter_cell: &RefCell<RJiter>, writer_cell: &RefCell<dyn Write>| {
                    let mut rjiter = rjiter_cell.borrow_mut();
                    let mut writer = writer_cell.borrow_mut();
                    let peek = rjiter.peek().unwrap();
                    write!(writer, "(matched {:?})", peek).unwrap();
                    StreamOp::None
                },
            );
            Some(action)
        } else {
            None
        }
    };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<dyn Write>> { None };

    let result = scan(
        find_action,
        find_end_action,
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

    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<dyn Write>> {
        if iter_match(
            || ["#atom".as_bytes(), "#array".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        ) {
            Some(Box::new(
                |rjiter_cell: &RefCell<RJiter>, writer_cell: &RefCell<dyn Write>| {
                    let mut rjiter = rjiter_cell.borrow_mut();
                    let mut writer = writer_cell.borrow_mut();
                    let peek = rjiter.peek().unwrap();
                    write!(writer, "(matched {:?})", peek).unwrap();
                    StreamOp::None
                },
            ))
        } else {
            None
        }
    };
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<dyn Write>> { None };

    let result = scan(
        find_action,
        find_end_action,
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

    let fields = vec!['a', 'b', 'c', 'd', 'e', 'f'];
    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<dyn Write>> {
        for field in &fields {
            let field_str = field.to_string();
            if iter_match(
                || ["#atom".as_bytes(), field_str.as_bytes(), "#top".as_bytes()],
                structural_pseudoname,
                context.clone(),
            ) {
                return Some(Box::new(handle_atom));
            }
        }
        None
    };
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<dyn Write>> { None };

    let result = scan(
        find_action,
        find_end_action,
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

    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<dyn Write>> {
        if iter_match(
            || ["#atom".as_bytes(), "#top".as_bytes()],
            structural_pseudoname,
            context,
        ) {
            Some(Box::new(
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
            ))
        } else {
            None
        }
    };
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<dyn Write>> { None };

    let result = scan(
        find_action,
        find_end_action,
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

    let find_action = |structural_pseudoname: StructuralPseudoname,
                       context: ContextIter|
     -> Option<BoxedAction<dyn Write>> {
        if iter_match(
            || ["message".as_bytes()],
            structural_pseudoname,
            context.clone(),
        ) {
            Some(Box::new(
                |_: &RefCell<RJiter>, writer: &RefCell<dyn Write>| {
                    let result = writer.borrow_mut().write_all(b"(new message)\n");
                    match result {
                        Ok(_) => StreamOp::None,
                        Err(e) => StreamOp::Error(Box::new(e)),
                    }
                },
            ))
        } else if iter_match(|| ["content".as_bytes()], structural_pseudoname, context) {
            Some(Box::new(
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
            ))
        } else {
            None
        }
    };
    let find_end_action = |structural_pseudoname: StructuralPseudoname,
                           context: ContextIter|
     -> Option<BoxedEndAction<dyn Write>> {
        if iter_match(|| ["message".as_bytes()], structural_pseudoname, context) {
            Some(Box::new(|writer: &RefCell<dyn Write>| {
                writer.borrow_mut().write_all(b"\n")?;
                Ok(())
            }))
        } else {
            None
        }
    };

    scan(
        find_action,
        find_end_action,
        &RefCell::new(rjiter),
        &writer_cell,
        &mut scan_stack,
        {
            let sse_tokens: &[&[u8]] = &[b"data:", b"DONE"];
            &Options::with_sse_tokens(sse_tokens)
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

    // find_action that never matches anything
    let find_action = |_structural_pseudoname: StructuralPseudoname,
                       _context: ContextIter|
     -> Option<BoxedAction<()>> { None };
    // find_end_action that never matches anything
    let find_end_action = |_structural_pseudoname: StructuralPseudoname,
                           _context: ContextIter|
     -> Option<BoxedEndAction<()>> { None };

    scan(
        find_action,
        find_end_action,
        &rjiter_cell,
        &RefCell::new(()),
        &mut scan_stack,
        &Options::new(),
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

    for _ in 0..4 {
        scan(
            find_action,
            find_end_action,
            &rjiter_cell,
            &RefCell::new(()),
            &mut scan_stack,
            &Options {
                sse_tokens: &[],
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
