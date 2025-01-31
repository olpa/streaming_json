use crate::trigger::{find_action, Trigger};
use crate::trigger::{BoxedAction, BoxedEndAction};
use rjiter::jiter::Peek;
use rjiter::RJiter;
use std::cell::RefCell;
use std::io;

#[derive(Debug, PartialEq)]
pub enum ActionResult {
    Ok,
    OkValueIsConsumed,
}

#[derive(Debug)]
pub struct ContextFrame {
    pub(crate) current_key: String,
    is_in_object: bool,
    is_in_array: bool,
    is_object_begin: bool,
}

fn handle_object<T>(
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
    triggers: &[Trigger<BoxedAction<T>>],
    triggers_end: &[Trigger<BoxedEndAction<T>>],
    mut cur_level: ContextFrame,
    context: &mut Vec<ContextFrame>,
) -> (ActionResult, ContextFrame) {
    {
        let mut rjiter = rjiter_cell.borrow_mut();
        let keyr = if cur_level.is_object_begin {
            rjiter.next_object()
        } else {
            rjiter.next_key()
        };
        cur_level.is_object_begin = false;

        let key = keyr.unwrap();
        if key.is_none() {
            let cur_level = context.pop().unwrap();
            if let Some(end_action) = find_action(triggers_end, &cur_level.current_key, context) {
                end_action(baton_cell);
            }
            return (ActionResult::OkValueIsConsumed, cur_level);
        }

        let key_str = key.unwrap().to_string();
        cur_level.current_key = key_str;
    }

    if let Some(action) = find_action(triggers, &cur_level.current_key, context) {
        return (action(rjiter_cell, baton_cell), cur_level);
    }
    (ActionResult::Ok, cur_level)
}

fn handle_array(
    rjiter: &mut RJiter,
    mut cur_level: ContextFrame,
    context: &mut Vec<ContextFrame>,
) -> (Option<Peek>, ContextFrame) {
    let apickedr = if cur_level.is_object_begin {
        rjiter.known_array()
    } else {
        rjiter.array_step()
    };
    cur_level.is_object_begin = false;

    let peeked = apickedr.unwrap();
    if peeked.is_none() {
        let new_cur_level = context.pop().unwrap();
        return (None, new_cur_level);
    }
    (peeked, cur_level)
}

#[allow(clippy::missing_panics_doc)]
pub fn scan_json<T>(
    triggers: &[Trigger<BoxedAction<T>>],
    triggers_end: &[Trigger<BoxedEndAction<T>>],
    sse_tokens: &[&str],
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
) {
    let mut context: Vec<ContextFrame> = Vec::new();
    let mut cur_level = ContextFrame {
        current_key: "#top".to_string(),
        is_object_begin: false,
        is_in_object: false,
        is_in_array: false,
    };

    'main_loop: loop {
        let mut peeked = None;

        if cur_level.is_in_object {
            let (action_result, new_cur_level) = handle_object(
                rjiter_cell,
                baton_cell,
                triggers,
                triggers_end,
                cur_level,
                &mut context,
            );
            cur_level = new_cur_level;

            if action_result == ActionResult::OkValueIsConsumed {
                continue;
            }
        }

        let mut rjiter = rjiter_cell.borrow_mut();

        if cur_level.is_in_array {
            let (arr_peeked, new_cur_level) = handle_array(&mut rjiter, cur_level, &mut context);
            cur_level = new_cur_level;

            if arr_peeked.is_none() {
                continue;
            }
            peeked = arr_peeked;
        }

        if peeked.is_none() {
            let peekedr = rjiter.peek();
            if let Err(rjiter::Error {
                error_type:
                    rjiter::error::ErrorType::JsonError(
                        rjiter::jiter::JsonErrorType::EofWhileParsingValue,
                    ),
                ..
            }) = peekedr
            {
                assert!(context.is_empty(), "scan_json: eof while inside an object");
                let eof = rjiter.finish();
                eof.unwrap();
                break;
            }

            peeked = Some(peekedr.unwrap());
        };

        let peeked = peeked.unwrap();

        if peeked == Peek::Array {
            context.push(cur_level);
            cur_level = ContextFrame {
                current_key: "#array".to_string(),
                is_in_array: true,
                is_in_object: false,
                is_object_begin: true,
            };
            continue;
        }

        if peeked == Peek::Object {
            context.push(cur_level);
            cur_level = ContextFrame {
                current_key: "#object".to_string(),
                is_in_object: true,
                is_in_array: false,
                is_object_begin: true,
            };
            continue;
        }

        if peeked == Peek::Null {
            rjiter.known_null().unwrap();
            continue;
        }
        if peeked == Peek::True {
            rjiter.known_bool(peeked).unwrap();
            continue;
        }
        if peeked == Peek::False {
            rjiter.known_bool(peeked).unwrap();
            continue;
        }
        if peeked == Peek::String {
            rjiter.write_long_bytes(&mut io::sink()).unwrap();
            continue;
        }

        let maybe_number = rjiter.next_number();
        if maybe_number.is_ok() {
            continue;
        }

        // If we are at the top level, we need to drop the SSE tokens
        // The array condition is to handle the token "[DONE]", which is
        // parsed as an array with one element, the string "DONE".
        if context.is_empty() || (cur_level.is_in_array && context.len() == 1) {
            for sse_token in sse_tokens {
                let found = rjiter.known_skip_token(sse_token.as_bytes());
                if found.is_ok() {
                    continue 'main_loop;
                }
            }
        }

        panic!("scan_json: unhandled: peeked={peeked:?}");
    }
}
