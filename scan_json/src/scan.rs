//! Implementation of the `scan` function to scan a JSON stream.

use crate::error::Error as ScanError;
use crate::error::Result as ScanResult;
use crate::matcher::{Action, EndAction, StreamOp, StructuralPseudoname};
use crate::stack::ContextIter;
use embedded_io::{Read, Write};
use rjiter::jiter::Peek;
use rjiter::RJiter;

/// A sink writer that discards all written data
struct Sink;

impl embedded_io::ErrorType for Sink {
    type Error = embedded_io::ErrorKind;
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
use u8pool::{U8Pool, U8PoolError};

/// Options for configuring the scan behavior
#[derive(Debug)]
pub struct Options<'options> {
    /// Slice of SSE tokens to ignore at the top level
    pub sse_tokens: &'options [&'options [u8]],
    /// Whether to stop scanning as soon as possible, or scan the complete JSON stream
    pub stop_early: bool,
}

impl<'options> Options<'options> {
    #[allow(clippy::new_without_default)]
    #[must_use]
    /// Creates new default options with no SSE tokens
    pub fn new() -> Self {
        Self {
            sse_tokens: &[],
            stop_early: false,
        }
    }

    #[must_use]
    /// Creates options with specified SSE tokens
    pub fn with_sse_tokens(tokens: &'options [&'options [u8]]) -> Self {
        Self {
            sse_tokens: tokens,
            stop_early: false,
        }
    }
}

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
#[allow(clippy::too_many_lines)]
fn handle_object<B: Copy, R: Read>(
    rjiter: &mut RJiter<R>,
    baton: B,
    find_action: &impl Fn(StructuralPseudoname, ContextIter, B) -> Option<Action<B, R>>,
    find_end_action: &impl Fn(StructuralPseudoname, ContextIter, B) -> Option<EndAction<B>>,
    position: StructurePosition,
    context: &mut U8Pool,
) -> ScanResult<StructurePosition> {
    //
    // Call the begin-trigger for the object
    //
    if position == StructurePosition::ObjectBegin {
        if let Some(begin_action) = find_action(
            StructuralPseudoname::Object,
            ContextIter::new(context),
            baton,
        ) {
            match begin_action(rjiter, baton) {
                StreamOp::None => (),
                StreamOp::Error(message) => {
                    return Err(ScanError::ActionError {
                        message,
                        position: rjiter.current_index(),
                    })
                }
                StreamOp::ValueIsConsumed => {
                    #[allow(unsafe_code)]
                    return Ok(*unsafe { context.top_assoc_obj::<StructurePosition>() }
                        .ok_or_else(|| ScanError::InternalError {
                            position: rjiter.current_index(),
                            message: "Context stack is empty when handling ValueIsConsumed",
                        })?);
                }
            }
        }
    }

    //
    // Call the end-trigger for the previous key
    //
    if position != StructurePosition::ObjectBegin {
        let end_action =
            find_end_action(StructuralPseudoname::None, ContextIter::new(context), baton);
        #[allow(unsafe_code)]
        let _ = unsafe { context.pop_assoc::<StructurePosition>() };
        if let Some(end_action) = end_action {
            if let Err(message) = end_action(baton) {
                return Err(ScanError::ActionError {
                    message,
                    position: rjiter.current_index(),
                });
            }
        }
    }

    //
    // Find the next key in the object or the end of the object
    //
    let keyr = if position == StructurePosition::ObjectBegin {
        rjiter.next_object_bytes()
    } else {
        rjiter.next_key_bytes()
    }?;

    match keyr {
        None => {
            //
            // Call the end-trigger for the object
            //
            if let Some(end_action) = find_end_action(
                StructuralPseudoname::Object,
                ContextIter::new(context),
                baton,
            ) {
                if let Err(message) = end_action(baton) {
                    return Err(ScanError::ActionError {
                        message,
                        position: rjiter.current_index(),
                    });
                }
            }
            #[allow(unsafe_code)]
            return Ok(
                *unsafe { context.top_assoc_obj::<StructurePosition>() }.ok_or_else(|| {
                    ScanError::InternalError {
                        position: rjiter.current_index(),
                        message: "Context stack is empty when ending object",
                    }
                })?,
            );
        }
        Some(key) => {
            //
            // Remember the current key
            //
            context
                .push_assoc(StructurePosition::ObjectMiddle, key)
                .map_err(|e| match e {
                    U8PoolError::SliceLimitExceeded { max_slices } => {
                        ScanError::MaxNestingExceeded {
                            position: rjiter.current_index(),
                            level: max_slices,
                        }
                    }
                    _ => ScanError::InternalError {
                        position: rjiter.current_index(),
                        message: "Failed to push key to context pool",
                    },
                })?;
        }
    }

    //
    // Execute the action for the current key
    //
    if let Some(action) = find_action(StructuralPseudoname::None, ContextIter::new(context), baton)
    {
        match action(rjiter, baton) {
            StreamOp::Error(message) => {
                return Err(ScanError::ActionError {
                    message,
                    position: rjiter.current_index(),
                });
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
fn handle_array<B: Copy, R: Read>(
    rjiter: &mut RJiter<R>,
    baton: B,
    find_action: &impl Fn(StructuralPseudoname, ContextIter, B) -> Option<Action<B, R>>,
    find_end_action: &impl Fn(StructuralPseudoname, ContextIter, B) -> Option<EndAction<B>>,
    position: StructurePosition,
    context: &mut U8Pool,
) -> ScanResult<(Option<Peek>, StructurePosition)> {
    //
    // Call the begin-trigger at the beginning of the array
    //
    if position == StructurePosition::ArrayBegin {
        if let Some(begin_action) = find_action(
            StructuralPseudoname::Array,
            ContextIter::new(context),
            baton,
        ) {
            match begin_action(rjiter, baton) {
                StreamOp::None => (),
                StreamOp::ValueIsConsumed => {
                    return Ok((
                        None,
                        #[allow(unsafe_code)]
                        *unsafe { context.top_assoc_obj::<StructurePosition>() }.ok_or_else(
                            || ScanError::InternalError {
                                position: rjiter.current_index(),
                                message:
                                    "Context stack is empty when handling ValueIsConsumed in array",
                            },
                        )?,
                    ));
                }
                StreamOp::Error(message) => {
                    return Err(ScanError::ActionError {
                        message,
                        position: rjiter.current_index(),
                    });
                }
            }
        }

        // Push to context with position "middle in array" and name "#array"
        if context
            .push_assoc(StructurePosition::ArrayMiddle, b"#array")
            .is_err()
        {
            return Err(ScanError::MaxNestingExceeded {
                position: rjiter.current_index(),
                level: context.len(),
            });
        }
    }

    //
    // Get the next item in the array
    //
    let peeked = if position == StructurePosition::ArrayBegin {
        rjiter.known_array()
    } else {
        rjiter.array_step()
    }?;

    //
    // If at the end of the array
    //
    if peeked.is_none() {
        //
        // Pop the context before calling the end-trigger
        //
        #[allow(unsafe_code)]
        unsafe { context.pop_assoc::<StructurePosition>() }.ok_or_else(|| {
            ScanError::InternalError {
                position: rjiter.current_index(),
                message: "Context stack is empty when ending array",
            }
        })?;

        //
        // Call the end-trigger
        //
        if let Some(end_action) = find_end_action(
            StructuralPseudoname::Array,
            ContextIter::new(context),
            baton,
        ) {
            if let Err(message) = end_action(baton) {
                return Err(ScanError::ActionError {
                    message,
                    position: rjiter.current_index(),
                });
            }
        }
        return Ok((
            None,
            #[allow(unsafe_code)]
            *unsafe { context.top_assoc_obj::<StructurePosition>() }.ok_or_else(|| {
                ScanError::InternalError {
                    position: rjiter.current_index(),
                    message: "Context stack is empty when ending array",
                }
            })?,
        ));
    }
    Ok((peeked, StructurePosition::ArrayMiddle))
}

///
/// Skips over basic JSON values (null, true, false, numbers, strings)
///
fn skip_basic_values<R: Read>(peeked: Peek, rjiter: &mut RJiter<R>) -> ScanResult<()> {
    if peeked == Peek::String {
        rjiter.write_long_bytes(&mut Sink)?;
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
    if rjiter.next_number_bytes().is_ok() {
        return Ok(());
    }
    Err(ScanError::UnhandledPeek {
        peek: peeked,
        position: rjiter.current_index(),
    })
}

///
/// Parses JSON and executes callbacks based on patterns.
/// See `README.md` for examples of how to use this function.
///
/// # Arguments
///
/// * `find_action` - A matcher function that returns a callback for begin events
/// * `find_end_action` - A matcher function that returns a callback for end events
/// * `rjiter` - Mutable reference to the JSON iterator
/// * `baton` - Reference cell containing the caller's state
/// * `working_buffer` - Working buffer for the context stack
/// * `options` - Configuration options for scan behavior
///
/// # Matching and Actions
///
/// While parsing, `scan` maintains a context stack containing the path of element names
/// from the root to the current nesting level. See [`crate::iter_match()`] for details
/// about context and matching.
///
/// The workflow for each structural element:
///
/// 1. Call `find_action` and execute the returned callback if found
/// 2. If the element is an object or array, update the context and parse the next level
/// 3. Call `find_end_action` and execute the returned callback if found
///
/// If in step 1 an action returns `StreamOp::ValueIsConsumed`, the `scan` function
/// skips the remaining steps, assuming the action correctly advanced the parser.
///
/// # Baton (State) Patterns and Side Effects
///
/// Actions receive two arguments:
///
/// - `rjiter`: Mutable reference to the `RJiter` parser object that actions can use to consume JSON values
/// - `baton`: State object for side effects, which can be either:
///   - **Simple baton**: Any `Copy` type (like `i32`, `bool`, `()`) passed by value for read-only or stateless operations
///   - **`RefCell` baton**: `&RefCell<B>` for mutable state that needs to be shared across action calls
///
/// # Error Handling in Actions
///
/// When an action encounters an error, it returns `StreamOp::Error(message)` with a static string message.
/// The `scan` function converts this to an `ActionError` with the message and position.
///
/// If handlers need to preserve detailed errors (like IO error kinds from `embedded_io::ErrorKind`
/// or specific `RJiter` error details), they must store them in their baton and retrieve them after `scan()` returns.
/// The `StreamOp::Error` message is only a generic indicator that an error occurred.
///
/// See the `idtransform` implementation for an example of storing detailed errors in the baton and retrieving
/// them after `scan()` completes.
///
/// # Working Buffer Sizing
///
/// The working buffer should be sized based on expected nesting depth, average key
/// length, and 8 bytes per context frame. A reasonable default for most applications:
///
/// - 512 bytes and 20 nesting levels with 16-byte average key names
///
/// # Options
///
/// - `sse_tokens`: Tokens to ignore at the top level, useful for server-side
///   events tokens like `data:` or `[DONE]`
/// - `stop_early`: By default, `scan` processes multiple JSON objects (like JSONL format).
///   Set to `true` to stop after the first complete element
///
/// # Errors
///
/// Returns any error from [`crate::error::Error`].
///
#[allow(clippy::too_many_lines, clippy::elidable_lifetime_names)]
pub fn scan<'options, B: Copy, R: Read>(
    find_action: impl Fn(StructuralPseudoname, ContextIter, B) -> Option<Action<B, R>>,
    find_end_action: impl Fn(StructuralPseudoname, ContextIter, B) -> Option<EndAction<B>>,
    rjiter: &mut RJiter<R>,
    baton: B,
    working_buffer: &mut U8Pool,
    options: &Options<'options>,
) -> ScanResult<()> {
    let context = working_buffer; // Alias for better readability in function body

    let mut position = StructurePosition::Top;
    context
        .push_assoc(position, b"#top")
        .map_err(|_e| ScanError::MaxNestingExceeded {
            position: rjiter.current_index(),
            level: 0,
        })?;

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
        if position == StructurePosition::ObjectBegin || position == StructurePosition::ObjectMiddle
        {
            match handle_object(
                rjiter,
                baton,
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
                rjiter,
                baton,
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
                Ok((_peeked_val, _unexpected)) => {
                    return Err(ScanError::InternalError {
                        position: rjiter.current_index(),
                        message: "Unexpected position from handle_array",
                    });
                }
                Err(e) => return Err(e),
            }
        }

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
                rjiter.finish()?;
                break;
            }

            peeked = Some(peekedr?);
        }

        let peeked = peeked.ok_or(ScanError::InternalError {
            position: rjiter.current_index(),
            message: "peeked is none when it should not be",
        })?;
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
        let action = find_action(StructuralPseudoname::Atom, ContextIter::new(context), baton);
        if let Some(action) = action {
            match action(rjiter, baton) {
                StreamOp::Error(message) => {
                    return Err(ScanError::ActionError {
                        message,
                        position: rjiter.current_index(),
                    })
                }
                StreamOp::ValueIsConsumed => continue 'main_loop,
                StreamOp::None => (),
            }
        }

        if skip_basic_values(peeked, rjiter).is_ok() {
            continue;
        }

        //
        // If we are at the top level, we need to drop the SSE tokens
        // The array condition is to handle the token "[DONE]", which is
        // parsed as an array with one element, the string "DONE".
        //
        if (position == StructurePosition::Top)
            || ((position == StructurePosition::ArrayBegin
                || position == StructurePosition::ArrayMiddle)
                && context.len() == 2)
        {
            for sse_token in options.sse_tokens {
                if rjiter.known_skip_token(sse_token).is_ok() {
                    continue 'main_loop;
                }
            }
        }

        return Err(ScanError::UnhandledPeek {
            peek: peeked,
            position: rjiter.current_index(),
        });
    }

    Ok(())
}
