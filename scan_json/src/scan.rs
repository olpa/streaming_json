//! Implementation of the `scan` function to scan a JSON stream.

use crate::action::{BoxedAction, BoxedEndAction, StreamOp};
use crate::error::Error as ScanError;
use crate::error::Result as ScanResult;
use rjiter::jiter::Peek;
use rjiter::RJiter;
use std::cell::RefCell;
use std::io;
use u8pool::U8Pool;


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

use crate::stack::ContextIter;
use crate::matcher::StructuralPseudoname;

/// Position in the JSON structure during scanning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructurePosition {
    /// At the top level of the JSON document
    Top,
    /// At the beginning of an object (just opened)
    ObjectBegin,
    /// In the middle of an object
    ObjectMiddle,
    /// Between key and value in an object
    ObjectBetweenKV,
    /// At the beginning of an array (just opened)
    ArrayBegin,
    /// In the middle of an array
    ArrayMiddle,
}


// Handle a JSON object key
//
// - Call the begin-action for the object
// - Call the end-action for the previous key
// - Find the next key in the object or the end of the object
// - Push the current key onto the context stack, or call the end-action for the object
// - Call the begin-action for the current key
//
// Stack:
// - On the first key, push it
// - On subsequent keys, pop the previous key, push the current key
// - On end of object, pop the last key
// - Contract: The stack state after the end of the object is the same as before the begin of the object.
//   The returned StructurePosition after the end is one from the top of the stack before the begin of the object.
fn handle_object<T: ?Sized>(
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
    find_action: &impl Fn(StructuralPseudoname, ContextIter) -> Option<BoxedAction<T>>,
    find_end_action: &impl Fn(StructuralPseudoname, ContextIter) -> Option<BoxedEndAction<T>>,
    position: StructurePosition,
    context: &mut U8Pool,
) -> ScanResult<StructurePosition>
{
    //
    // Call the begin-trigger for the object
    //
    if position == StructurePosition::ObjectBegin {
        if let Some(begin_action) = find_action(StructuralPseudoname::Object, ContextIter::new(context)) {
            match begin_action(rjiter_cell, baton_cell) {
                StreamOp::None => (),
                StreamOp::Error(e) => {
                    return Err(ScanError::ActionError(
                        e,
                        rjiter_cell.borrow().current_index(),
                    ))
                }
                StreamOp::ValueIsConsumed => {
                    return Ok(*context.top_assoc_obj::<StructurePosition>()
                        .ok_or_else(|| ScanError::InternalError(
                            rjiter_cell.borrow().current_index(),
                            "Context stack is empty when handling ValueIsConsumed".to_string(),
                        ))?);
                }
            }
        }
    }

    //
    // Call the end-trigger for the previous key
    //
    if position != StructurePosition::ObjectBegin {
        let end_action = find_end_action(StructuralPseudoname::None, ContextIter::new(context));
        let _ = context.pop_assoc::<StructurePosition>();
        if let Some(end_action) = end_action {
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
    let keyr = if position == StructurePosition::ObjectBegin {
        rjiter.next_object()
    } else {
        rjiter.next_key()
    }?;

    match keyr {
        None => {
            //
            // Call the end-trigger for the object
            //
            if let Some(end_action) = find_end_action(StructuralPseudoname::Object, ContextIter::new(context)) {
                if let Err(e) = end_action(baton_cell) {
                    return Err(ScanError::ActionError(
                        e,
                        rjiter.current_index(),
                    ));
                }
            }
            return Ok(*context.top_assoc_obj::<StructurePosition>()
                .ok_or_else(|| ScanError::InternalError(
                    rjiter.current_index(),
                    "Context stack is empty when ending object".to_string(),
                ))?);
        }
        Some(key) => {
            //
            // Remember the current key
            //
            context.push_assoc(StructurePosition::ObjectMiddle, key.as_bytes())
                .map_err(|_| ScanError::InternalError(
                    rjiter.current_index(),
                    "Failed to push key to context pool".to_string()
                ))?;
        }
    }

    //
    // Execute the action for the current key
    //
    if let Some(action) = find_action(StructuralPseudoname::None, ContextIter::new(context)) {
        drop(rjiter);
        match action(rjiter_cell, baton_cell) {
            StreamOp::Error(e) => {
                return Err(ScanError::ActionError(
                    e,
                    rjiter_cell.borrow().current_index(),
                ));
            }
            StreamOp::ValueIsConsumed => {
                return Ok(StructurePosition::ObjectMiddle);
            }
            StreamOp::None => (),
        }
    }

    Ok(StructurePosition::ObjectBetweenKV)
}

// Handle a JSON array item.
//
// - If at the beginning of the array
//   - Call the begin-trigger
//   - Push "#array" to the context stack
// - Return the next item in the array
// - If at the end of the array
//   - Pop the "#array" from the context stack
//   - Call the end-trigger
//
// Stack:
// - On the first item, push "#array"
// - On subsequent items, do nothing
// - On end of array, pop "#array"
// - Contract: The stack state after the end of the array is the same as before the begin of the array.
//   The returned StructurePosition after the end is one from the top of the stack before the begin of the array.
//
fn handle_array<T: ?Sized>(
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
    find_action: &impl Fn(StructuralPseudoname, ContextIter) -> Option<BoxedAction<T>>,
    find_end_action: &impl Fn(StructuralPseudoname, ContextIter) -> Option<BoxedEndAction<T>>,
    position: StructurePosition,
    context: &mut U8Pool,
) -> ScanResult<(Option<Peek>, StructurePosition)>
{
    //
    // Call the begin-trigger at the beginning of the array
    //
    if position == StructurePosition::ArrayBegin {
        if let Some(begin_action) = find_action(StructuralPseudoname::Array, ContextIter::new(context)) {
            match begin_action(rjiter_cell, baton_cell) {
                StreamOp::None => (),
                StreamOp::ValueIsConsumed => {
                    return Ok((None, *context.top_assoc_obj::<StructurePosition>()
                        .ok_or_else(|| ScanError::InternalError(
                            rjiter_cell.borrow().current_index(),
                            "Context stack is empty when handling ValueIsConsumed in array".to_string(),
                        ))?));
                }
                StreamOp::Error(e) => {
                    return Err(ScanError::ActionError(e, rjiter_cell.borrow().current_index()));
                }
            }
        }

        // Push to context with position "middle in array" and name "#array"
        if context.push_assoc(StructurePosition::ArrayMiddle, b"#array").is_err() {
            return Err(ScanError::MaxNestingExceeded(
                rjiter_cell.borrow().current_index(),
                context.len(),
            ));
        }
    }

    //
    // Get the next item in the array
    //
    let peeked = if position == StructurePosition::ArrayBegin {
        let mut rjiter = rjiter_cell.borrow_mut();
        rjiter.known_array()
    } else {
        let mut rjiter = rjiter_cell.borrow_mut();
        rjiter.array_step()
    }?;

    //
    // If at the end of the array
    //
    if peeked.is_none() {
        //
        // Pop the context before calling the end-trigger
        //
        context.pop_assoc::<StructurePosition>()
            .ok_or_else(|| ScanError::InternalError(
                rjiter_cell.borrow().current_index(),
                "Context stack is empty when ending array".to_string(),
            ))?;

        //
        // Call the end-trigger
        //
        if let Some(end_action) = find_end_action(StructuralPseudoname::Array, ContextIter::new(context)) {
            if let Err(e) = end_action(baton_cell) {
                return Err(ScanError::ActionError(e, rjiter_cell.borrow().current_index()));
            }
        }
        return Ok((None, *context.top_assoc_obj::<StructurePosition>()
            .ok_or_else(|| ScanError::InternalError(
                rjiter_cell.borrow().current_index(),
                "Context stack is empty when ending array".to_string(),
            ))?));
    }
    Ok((peeked, StructurePosition::ArrayMiddle))
}

///
/// Skips over basic JSON values (null, true, false, numbers, strings)
///
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
pub fn scan<T: ?Sized>(
    find_action: impl Fn(StructuralPseudoname, ContextIter) -> Option<BoxedAction<T>>,
    find_end_action: impl Fn(StructuralPseudoname, ContextIter) -> Option<BoxedEndAction<T>>,
    rjiter_cell: &RefCell<RJiter>,
    baton_cell: &RefCell<T>,
    working_buffer: &mut U8Pool,
    options: &Options,
) -> ScanResult<()>
{
    let context = working_buffer; // Alias for better readability in function body

    let mut position = StructurePosition::Top;
    context.push_assoc(position, b"#top")
        .map_err(|_e| ScanError::MaxNestingExceeded(rjiter_cell.borrow().current_index(), 0))?;

    let mut is_progressed = false;

    'main_loop: loop {
        if is_progressed && options.stop_early && position == StructurePosition::Top {
            break;
        }
        is_progressed = true;

        let mut peeked = None;

        //
        // Handle object states
        //
        if position == StructurePosition::ObjectBegin || position == StructurePosition::ObjectMiddle {
            match handle_object(
                rjiter_cell,
                baton_cell,
                &find_action,
                &find_end_action,
                position,
                context,
            ) {
                Ok(new_position) => {
                    position = new_position;
                    continue 'main_loop;
                }
                Err(e) => return Err(e),
            }
        }

        //
        // Handle array states
        //
        if position == StructurePosition::ArrayBegin || position == StructurePosition::ArrayMiddle {
            match handle_array(
                rjiter_cell,
                baton_cell,
                &find_action,
                &find_end_action,
                position,
                context,
            ) {
                Ok((Some(arr_peeked), StructurePosition::ArrayMiddle)) => {
                    position = StructurePosition::ArrayMiddle;
                    peeked = Some(arr_peeked);
                    // Continue inside the loop to process the array item
                }
                Ok((None, new_position)) => {
                    // handle_array returned the position from the stack
                    position = new_position;
                    continue 'main_loop;
                }
                Ok((peeked_val, unexpected)) => {
                    return Err(ScanError::InternalError(
                        rjiter_cell.borrow().current_index(),
                        format!("Unexpected position from handle_array: {:?} with peeked: {:?}", unexpected, peeked_val),
                    ));
                }
                Err(e) => return Err(e),
            }
        }

        let mut rjiter = rjiter_cell.borrow_mut();

        //
        // Peek the next JSON value, handling end of the input
        // The only case we have a value already is when we are continuing from the `handle_array` block
        //
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
                if position != StructurePosition::Top {
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
        if position == StructurePosition::ObjectBetweenKV {
            position = StructurePosition::ObjectMiddle;
        }

        //
        // Branch based on the type of the peeked value
        //
        if peeked == Peek::Array {
            position = StructurePosition::ArrayBegin;
            continue 'main_loop;
        }
        if peeked == Peek::Object {
            position = StructurePosition::ObjectBegin;
            continue 'main_loop;
        }

        //
        // Call the action for the atom, then
        // - return if an error
        // - continue to the main loop if value is consumed, or
        // - pass through to the default handler
        //
        let action = find_action(StructuralPseudoname::Atom, ContextIter::new(context));
        if let Some(action) = action {
            drop(rjiter);
            let action_result = action(rjiter_cell, baton_cell);
            rjiter = rjiter_cell.borrow_mut();

            match action_result {
                StreamOp::Error(e) => {
                    return Err(ScanError::ActionError(e, rjiter.current_index()))
                }
                StreamOp::ValueIsConsumed => continue 'main_loop,
                StreamOp::None => (),
            }
        }

        if skip_basic_values(peeked, &mut rjiter).is_ok() {
            continue;
        }

        //
        // If we are at the top level, we need to drop the SSE tokens
        // The array condition is to handle the token "[DONE]", which is
        // parsed as an array with one element, the string "DONE".
        //
        if (position == StructurePosition::Top) || ((position == StructurePosition::ArrayBegin || position == StructurePosition::ArrayMiddle) && context.len() == 2) {
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
