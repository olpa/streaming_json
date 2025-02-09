//! Implementation of the `scan` function to scan a JSON stream.

use crate::action::{find_action, BoxedAction, BoxedEndAction, StreamOp, Trigger};
use crate::error::Error as ScanError;
use crate::error::Result as ScanResult;
use rjiter::jiter::Peek;
use rjiter::RJiter;
use std::cell::RefCell;
use std::io;

/// Maximum allowed nesting level for JSON structures
const MAX_NESTING: usize = 20;

/// Represents the current parsing context within the JSON structure
#[derive(Debug)]
pub struct ContextFrame {
    pub current_key: String,
    is_in_object: bool,
    is_in_array: bool,
    is_elem_begin: bool,
}

/// Create a new `ContextFrame` for testing purposes
#[allow(clippy::must_use_candidate)]
pub fn mk_context_frame_for_test(current_key: String) -> ContextFrame {
    ContextFrame {
        current_key,
        is_in_object: false,
        is_in_array: false,
        is_elem_begin: false,
    }
}

// Handle a JSON object key
//
// - Call the end-trigger for the previous key
// - Call the action for the current key
// - Pop the context stack if the object is ended
// - Push the current key onto the context stack
fn handle_object<T: ?Sized>(
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
    triggers: &[Trigger<BoxedAction<T>>],
    triggers_end: &[Trigger<BoxedEndAction<T>>],
    mut cur_level: ContextFrame,
    context: &mut Vec<ContextFrame>,
) -> ScanResult<(StreamOp, ContextFrame)> {
    {
        //
        // Special case: a top-level object was started
        //
        if cur_level.is_elem_begin && context.len() == 1 {
            if let Some(begin_action) = find_action(triggers, "#top", context) {
                match begin_action(rjiter_cell, baton_cell) {
                    StreamOp::None => (),
                    StreamOp::Error(e) => return Err(ScanError::ActionError(e)),
                    StreamOp::ValueIsConsumed => {
                        return Err(ScanError::ActionError(
                            "ValueIsConsumed is not supported for #top actions".into(),
                        ));
                    }
                }
            }
        }

        //
        // Call the end-trigger for the previous key
        //
        if !cur_level.is_elem_begin {
            if let Some(end_action) = find_action(triggers_end, &cur_level.current_key, context) {
                if let Err(e) = end_action(baton_cell) {
                    return Err(ScanError::ActionError(e));
                }
            }
        }

        //
        // Find the next key in the object or the end of the object
        //
        let mut rjiter = rjiter_cell.borrow_mut();
        let keyr = if cur_level.is_elem_begin {
            rjiter.next_object()
        } else {
            rjiter.next_key()
        };
        cur_level.is_elem_begin = false;

        //
        // If there is a next key, update the current key and continue
        //
        if let Some(key) = keyr? {
            let key_str = key.to_string();
            cur_level.current_key = key_str;
        } else {
            //
            // End of the object: mutate the context and end the function
            //
            return match context.pop() {
                Some(cur_level) => {
                    //
                    // Special case: a top-level object is ended
                    //
                    if context.is_empty() {
                        if let Some(end_action) = find_action(triggers_end, "#top", context) {
                            if let Err(e) = end_action(baton_cell) {
                                return Err(ScanError::ActionError(e));
                            }
                        }
                    }
                    //
                    // Return the the main loop
                    //
                    Ok((StreamOp::ValueIsConsumed, cur_level))
                }
                None => Err(ScanError::UnbalancedJson(rjiter.current_index())),
            };
        }
    }

    //
    // Execute the action for the current key
    //
    if let Some(action) = find_action(triggers, &cur_level.current_key, context) {
        let action_result = action(rjiter_cell, baton_cell);
        return match action_result {
            StreamOp::Error(e) => Err(ScanError::ActionError(e)),
            StreamOp::None | StreamOp::ValueIsConsumed => Ok((action_result, cur_level)),
        };
    }
    Ok((StreamOp::None, cur_level))
}

// Handle a JSON array item.
// Pop the context stack if the array is ended.
fn handle_array(
    rjiter: &mut RJiter,
    mut cur_level: ContextFrame,
    context: &mut Vec<ContextFrame>,
) -> ScanResult<(Option<Peek>, ContextFrame)> {
    let apickedr = if cur_level.is_elem_begin {
        rjiter.known_array()
    } else {
        rjiter.array_step()
    };
    cur_level.is_elem_begin = false;

    let peeked = apickedr?;
    if peeked.is_none() {
        if let Some(new_cur_level) = context.pop() {
            return Ok((None, new_cur_level));
        }
        return Err(ScanError::UnbalancedJson(rjiter.current_index()));
    }
    Ok((peeked, cur_level))
}

/// Skips over basic JSON values (null, true, false, numbers)
fn skip_basic_values(peeked: Peek, rjiter: &mut RJiter) -> ScanResult<()> {
    if peeked == Peek::Null {
        rjiter.known_null()?;
        return Ok(());
    }
    if peeked == Peek::True {
        rjiter.known_bool(peeked)?;
        return Ok(());
    }
    if peeked == Peek::False {
        rjiter.known_bool(peeked)?;
        return Ok(());
    }
    let maybe_number = rjiter.next_number();
    if maybe_number.is_ok() {
        return Ok(());
    }
    Err(ScanError::UnhandledPeek(peeked))
}

/// Pushes a new context frame onto the context stack
fn push_context(
    context: &mut Vec<ContextFrame>,
    cur_level: ContextFrame,
    rjiter: &RJiter,
) -> ScanResult<()> {
    if context.len() >= MAX_NESTING {
        return Err(ScanError::MaxNestingExceeded(
            rjiter.current_index(),
            context.len(),
        ));
    }
    context.push(cur_level);
    Ok(())
}

/// Scan a JSON stream, executing actions based on matched triggers and
/// handling nested structures up to a maximum depth.
/// It also ignores SSE tokens at the top level.
///
/// See the documentation in `README.md` for an example of how to use this function.
///
/// # Arguments
///
/// * `triggers` - List of action triggers to execute on matching keys
/// * `triggers_end` - List of end action triggers to execute when a key is ended
/// * `sse_tokens` - List of SSE tokens to ignore at the top level
/// * `rjiter_cell` - Reference cell containing the JSON iterator
/// * `baton_cell` - Reference cell containing the caller's state
///
/// # Errors
///
/// * `ScanError` - A wrapper over `Rjiter` errors, over an error from a trigger actions, or over wrong JSON structure
pub fn scan<T: ?Sized>(
    triggers: &[Trigger<BoxedAction<T>>],
    triggers_end: &[Trigger<BoxedEndAction<T>>],
    sse_tokens: &[&str],
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
) -> ScanResult<()> {
    let mut context: Vec<ContextFrame> = Vec::new();
    let mut cur_level = ContextFrame {
        current_key: "#top".to_string(),
        is_elem_begin: false,
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
            )?;
            cur_level = new_cur_level;

            match action_result {
                StreamOp::ValueIsConsumed => continue,
                StreamOp::Error(e) => return Err(ScanError::ActionError(e)),
                StreamOp::None => (),
            }
        }

        let mut rjiter = rjiter_cell.borrow_mut();

        if cur_level.is_in_array {
            let (arr_peeked, new_cur_level) = handle_array(&mut rjiter, cur_level, &mut context)?;
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
                if !context.is_empty() {
                    return Err(ScanError::UnbalancedJson(rjiter.current_index()));
                }
                if rjiter.finish().is_err() {
                    return Err(ScanError::InternalError(
                        rjiter.current_index(),
                        "not eof when should be eof".to_string(),
                    ));
                }
                break;
            }

            peeked = Some(peekedr?);
        };

        let peeked = peeked.ok_or(ScanError::InternalError(
            rjiter.current_index(),
            "peeked is none when it should not be".to_string(),
        ))?;

        if peeked == Peek::Array {
            push_context(&mut context, cur_level, &rjiter)?;
            cur_level = ContextFrame {
                current_key: "#array".to_string(),
                is_in_array: true,
                is_in_object: false,
                is_elem_begin: true,
            };
            continue;
        }

        if peeked == Peek::Object {
            push_context(&mut context, cur_level, &rjiter)?;
            cur_level = ContextFrame {
                current_key: "#object".to_string(),
                is_in_array: false,
                is_in_object: true,
                is_elem_begin: true,
            };
            continue;
        }

        if peeked == Peek::String {
            rjiter.write_long_bytes(&mut io::sink())?;
            continue;
        }

        if skip_basic_values(peeked, &mut rjiter).is_ok() {
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

        return Err(ScanError::UnhandledPeek(peeked));
    }

    Ok(())
}
