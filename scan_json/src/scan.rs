//! Implementation of the `scan` function to scan a JSON stream.

use crate::action::{BoxedAction, BoxedEndAction, StreamOp};
use crate::error::Error as ScanError;
use crate::error::Result as ScanResult;
use rjiter::jiter::Peek;
use rjiter::RJiter;
use std::cell::RefCell;
use std::io;
use u8pool::U8Pool;

/// Type alias for context iterator (simple concrete type)
type ContextIterator<'context> = Vec<&'context [u8]>;

/// Options for configuring the scan behavior
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// List of SSE tokens to ignore at the top level
    pub sse_tokens: Vec<String>,
    /// Whether to stop scanning as soon as possible, or scan the complete JSON stream
    pub stop_early: bool,
}

impl Options {
    #[allow(clippy::must_use_candidate)]
    /// Creates new default options
    pub fn new() -> Self {
        Self::default()
    }
}

/// Metadata associated with each context frame in the `U8Pool` stack
#[derive(Debug, Clone, Copy)]
struct StateFrame {
    is_in_object: bool,
    is_in_array: bool,
    is_elem_begin: bool,
}

/// Helper function to build context iterator from U8Pool for matchers
fn build_context_iter<'a>(pool: &'a U8Pool) -> ContextIterator<'a> {
    pool.iter_assoc::<StateFrame>().map(|(_assoc, key_slice)| key_slice).collect()
}

// Handle a JSON object key
//
// - Call the end-trigger for the previous key
// - Call the action for the current key
// - Pop the context stack if the object is ended
// - Push the current key onto the context stack
fn handle_object<'a, FindAction, FindEndAction, T: ?Sized>(
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
    find_action: &FindAction,
    find_end_action: &FindEndAction,
    mut cur_level_frame: StateFrame,
    mut cur_level_key: Vec<u8>,
    context: &'a mut U8Pool,
) -> ScanResult<(StreamOp, StateFrame, Vec<u8>)>
where
    FindAction: for<'context> Fn(&[u8], ContextIterator<'context>) -> Option<BoxedAction<T>>,
    FindEndAction: for<'context> Fn(&[u8], ContextIterator<'context>) -> Option<BoxedEndAction<T>>,
{
    {
        //
        // Call the begin-trigger for the object
        //
        if cur_level_frame.is_elem_begin {
            if let Some(begin_action) = find_action(b"#object", build_context_iter(context)) {
                match begin_action(rjiter_cell, baton_cell) {
                    StreamOp::None => (),
                    StreamOp::Error(e) => {
                        return Err(ScanError::ActionError(
                            e,
                            rjiter_cell.borrow().current_index(),
                        ))
                    }
                    StreamOp::ValueIsConsumed => {
                        return Err(ScanError::ActionError(
                            "ValueIsConsumed is not supported for #top actions".into(),
                            rjiter_cell.borrow().current_index(),
                        ));
                    }
                }
            }
        }

        //
        // Call the end-trigger for the previous key
        //
        if !cur_level_frame.is_elem_begin {
            if let Some(end_action) = find_end_action(&cur_level_key, build_context_iter(context)) {
                if let Err(e) = end_action(baton_cell) {
                    return Err(ScanError::ActionError(
                        e,
                        rjiter_cell.borrow().current_index(),
                    ));
                }
            }
        }

        //
        // Find the next key in the object or the end of the object
        //
        let mut rjiter = rjiter_cell.borrow_mut();
        let keyr = if cur_level_frame.is_elem_begin {
            rjiter.next_object()
        } else {
            rjiter.next_key()
        };
        cur_level_frame.is_elem_begin = false;

        //
        // If there is a next key, update the current key and continue
        //
        if let Some(key) = keyr? {
            cur_level_key = key.as_bytes().to_vec();
        } else {
            //
            // Call the end-trigger for the object
            //
            if let Some(end_action) = find_end_action(b"#object", build_context_iter(context)) {
                if let Err(e) = end_action(baton_cell) {
                    return Err(ScanError::ActionError(
                        e,
                        rjiter_cell.borrow().current_index(),
                    ));
                }
            }
            //
            // End of the object: mutate the context and end the function
            //
            return match context.pop_assoc::<StateFrame>() {
                Some((frame, key_slice)) => Ok((StreamOp::ValueIsConsumed, *frame, key_slice.to_vec())),
                None => Err(ScanError::UnbalancedJson(rjiter.current_index())),
            };
        }
    }

    //
    // Execute the action for the current key
    //
    if let Some(action) = find_action(&cur_level_key, build_context_iter(context)) {
        let action_result = action(rjiter_cell, baton_cell);
        return match action_result {
            StreamOp::Error(e) => Err(ScanError::ActionError(
                e,
                rjiter_cell.borrow().current_index(),
            )),
            StreamOp::None | StreamOp::ValueIsConsumed => Ok((action_result, cur_level_frame, cur_level_key)),
        };
    }
    Ok((StreamOp::None, cur_level_frame, cur_level_key))
}

// Handle a JSON array item.
// Pop the context stack if the array is ended.
fn handle_array<FindAction, FindEndAction, T: ?Sized>(
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
    find_action: &FindAction,
    find_end_action: &FindEndAction,
    mut cur_level_frame: StateFrame,
    cur_level_key: Vec<u8>,
    context: &mut U8Pool,
) -> ScanResult<(Option<Peek>, StateFrame, Vec<u8>)>
where
    FindAction: for<'context> Fn(&[u8], ContextIterator<'context>) -> Option<BoxedAction<T>>,
    FindEndAction: for<'context> Fn(&[u8], ContextIterator<'context>) -> Option<BoxedEndAction<T>>,
{
    // Call the begin-trigger at the beginning of the array
    let mut is_array_consumed = false;
    if cur_level_frame.is_elem_begin {
        if let Some(begin_action) = find_action(b"#array", build_context_iter(context)) {
            match begin_action(rjiter_cell, baton_cell) {
                StreamOp::None => (),
                StreamOp::ValueIsConsumed => is_array_consumed = true,
                StreamOp::Error(e) => {
                    let rjiter = rjiter_cell.borrow();
                    return Err(ScanError::ActionError(e, rjiter.current_index()));
                }
            }
        }
    }

    // Get the next item in the array
    let apickedr = if is_array_consumed {
        Ok(None)
    } else if cur_level_frame.is_elem_begin {
        let mut rjiter = rjiter_cell.borrow_mut();
        rjiter.known_array()
    } else {
        let mut rjiter = rjiter_cell.borrow_mut();
        rjiter.array_step()
    };
    cur_level_frame.is_elem_begin = false;

    // Call the end-trigger at the end of the array
    let peeked = apickedr?;
    if peeked.is_none() {
        if let Some(end_action) = find_end_action(b"#array", build_context_iter(context)) {
            if let Err(e) = end_action(baton_cell) {
                let rjiter = rjiter_cell.borrow();
                return Err(ScanError::ActionError(e, rjiter.current_index()));
            }
        }
        if let Some((frame, key_slice)) = context.pop_assoc::<StateFrame>() {
            return Ok((None, *frame, key_slice.to_vec()));
        }
        let rjiter = rjiter_cell.borrow();
        return Err(ScanError::UnbalancedJson(rjiter.current_index()));
    }
    Ok((peeked, cur_level_frame, cur_level_key))
}

/// Skips over basic JSON values (null, true, false, numbers, strings)
fn skip_basic_values(peeked: Peek, rjiter: &mut RJiter) -> ScanResult<()> {
    if peeked == Peek::String {
        rjiter.write_long_bytes(&mut io::sink())?;
        return Ok(());
    }
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
    let maybe_number = rjiter.next_number_bytes();
    if maybe_number.is_ok() {
        return Ok(());
    }
    Err(ScanError::UnhandledPeek(peeked, rjiter.current_index()))
}

/// Pushes a new context frame onto the context stack
fn push_context(context: &mut U8Pool, cur_level_frame: StateFrame, cur_level_key: &[u8], rjiter: &RJiter) -> ScanResult<()> {
    let context_len = context.len();
    context
        .push_assoc(cur_level_frame, cur_level_key)
        .map_err(|_e| ScanError::MaxNestingExceeded(rjiter.current_index(), context_len))?;
    Ok(())
}

/// Scan a JSON stream, executing actions based on matched triggers and
/// handling nested structures. The caller provides a working buffer for
/// tracking the parsing context stack.
/// It also ignores SSE tokens at the top level.
///
/// See the documentation in `README.md` for an example of how to use this function.
///
/// # Working Buffer Size
///
/// The working buffer should be sized based on the expected nesting depth and
/// average key length of your JSON. A reasonable estimate:
/// - Average JSON key: 16 bytes
/// - Context metadata: 8 bytes per frame
/// - For 20 nesting levels: ~512 bytes working buffer
///
/// Use `U8Pool::with_default_max_slices(buffer)` for up to 32 nesting levels,
/// or `U8Pool::new(buffer, max_slices)` for custom limits.
///
/// # Arguments
///
/// * `triggers` - List of action triggers to execute on matching keys
/// * `triggers_end` - List of end action triggers to execute when a key is ended
/// * `rjiter_cell` - Reference cell containing the JSON iterator
/// * `baton_cell` - Reference cell containing the caller's state
/// * `working_buffer` - Working buffer for context stack (`U8Pool`)
/// * `options` - Configuration options for the scan behavior
///
/// # Errors
///
/// * `ScanError` - A wrapper over `Rjiter` errors, over an error from a trigger actions, or over wrong JSON structure
/// * `MaxNestingExceeded` - When the JSON nesting depth exceeds the working buffer capacity
#[allow(clippy::too_many_lines)]
pub fn scan<FindAction, FindEndAction, T: ?Sized>(
    find_action: FindAction,
    find_end_action: FindEndAction,
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
    working_buffer: &mut U8Pool,
    options: &Options,
) -> ScanResult<()>
where
    FindAction: for<'context> Fn(&[u8], ContextIterator<'context>) -> Option<BoxedAction<T>>,
    FindEndAction: for<'context> Fn(&[u8], ContextIterator<'context>) -> Option<BoxedEndAction<T>>,
{
    let context = working_buffer; // Alias for better readability in function body
    let mut cur_level_key_storage = Vec::from(b"#top" as &[u8]);
    let mut cur_level_frame = StateFrame {
        is_elem_begin: false,
        is_in_object: false,
        is_in_array: false,
    };

    let mut is_progressed = false;

    'main_loop: loop {
        if is_progressed && options.stop_early && context.is_empty() {
            break;
        }
        is_progressed = true;

        let mut peeked = None;

        if cur_level_frame.is_in_object {
            let (action_result, new_state_frame, new_key) = handle_object(
                rjiter_cell,
                baton_cell,
                &find_action,
                &find_end_action,
                cur_level_frame,
                cur_level_key_storage.clone(),
                context,
            )?;
            cur_level_frame = new_state_frame;
            cur_level_key_storage = new_key;

            match action_result {
                StreamOp::ValueIsConsumed => continue,
                StreamOp::Error(e) => {
                    return Err(ScanError::ActionError(
                        e,
                        rjiter_cell.borrow().current_index(),
                    ))
                }
                StreamOp::None => (),
            }
        }

        if cur_level_frame.is_in_array {
            let (arr_peeked, new_state_frame, new_key) = handle_array(
                rjiter_cell,
                baton_cell,
                &find_action,
                &find_end_action,
                cur_level_frame,
                cur_level_key_storage.clone(),
                context,
            )?;
            cur_level_frame = new_state_frame;
            cur_level_key_storage = new_key;

            if arr_peeked.is_none() {
                continue;
            }
            peeked = arr_peeked;
        }

        let mut rjiter = rjiter_cell.borrow_mut();

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
        }

        let peeked = peeked.ok_or(ScanError::InternalError(
            rjiter.current_index(),
            "peeked is none when it should not be".to_string(),
        ))?;

        if peeked == Peek::Array {
            push_context(context, cur_level_frame, &cur_level_key_storage, &rjiter)?;
            cur_level_key_storage = Vec::from(b"#array" as &[u8]);
            cur_level_frame = StateFrame {
                is_in_array: true,
                is_in_object: false,
                is_elem_begin: true,
            };
            continue;
        }

        if peeked == Peek::Object {
            push_context(context, cur_level_frame, &cur_level_key_storage, &rjiter)?;
            cur_level_key_storage = Vec::from(b"#object" as &[u8]);
            cur_level_frame = StateFrame {
                is_in_array: false,
                is_in_object: true,
                is_elem_begin: true,
            };
            continue;
        }

        // Handle basic (aka atomic) values
        push_context(context, cur_level_frame, &cur_level_key_storage, &rjiter)?;

        // Find action with current context
        let action = find_action(b"#atom", build_context_iter(context));

        // Pop the context we just pushed
        let (frame, key_slice) = context.pop_assoc::<StateFrame>().ok_or_else(|| {
            ScanError::InternalError(
                rjiter.current_index(),
                "Context stack is empty when it should not be".to_string(),
            )
        })?;
        cur_level_frame = *frame;
        cur_level_key_storage = key_slice.to_vec();

        // Call the action for the atom, then return (error) or continue (value is consumed)
        // or pass through to the default handler
        if let Some(action) = action {
            drop(rjiter);
            let action_result = action(rjiter_cell, baton_cell);
            rjiter = rjiter_cell.borrow_mut();

            match action_result {
                StreamOp::Error(e) => {
                    return Err(ScanError::ActionError(e, rjiter.current_index()))
                }
                StreamOp::ValueIsConsumed => continue,
                StreamOp::None => (),
            }
        }

        if skip_basic_values(peeked, &mut rjiter).is_ok() {
            continue;
        }

        // If we are at the top level, we need to drop the SSE tokens
        // The array condition is to handle the token "[DONE]", which is
        // parsed as an array with one element, the string "DONE".
        if context.is_empty() || (cur_level_frame.is_in_array && context.len() == 1) {
            for sse_token in &options.sse_tokens {
                let found = rjiter.known_skip_token(sse_token.as_bytes());
                if found.is_ok() {
                    continue 'main_loop;
                }
            }
        }

        return Err(ScanError::UnhandledPeek(peeked, rjiter.current_index()));
    }

    Ok(())
}
