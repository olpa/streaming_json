use crate::ConversionError;
use core::cell::RefCell;
use embedded_io::{Error as IoError, Read as IoRead, Write as IoWrite};
use rjiter::jiter::Peek;
use rjiter::RJiter;
use scan_json::matcher::StructuralPseudoname;
use scan_json::stack::ContextIter;
use scan_json::{scan, Action, EndAction, Options, StreamOp};
use u8pool::U8Pool;

/// Parse error without position information (used internally before position is known)
#[derive(Debug, Clone)]
struct ParseErrorNoPos {
    context: &'static str,
    /// Unknown type descriptor bytes (buffer, actual length used)
    unknown_type: Option<([u8; 32], usize)>,
}

impl ParseErrorNoPos {
    /// Convert to `ParseError` by adding position information
    fn with_position(self, position: usize) -> ConversionError {
        ConversionError::ParseError {
            position,
            context: self.context,
            unknown_type: self.unknown_type,
        }
    }
}

/// What phase of parsing we're in
///
/// Begin-transitions (when encountering the start of a key or value):
/// - From `ExpectingField`:
///    - Field key encountered -> `ExpectingTypeKey`
/// - From `ExpectingTypeKey`:
///    - Type key "M" -> `ExpectingField`
///    - Type key "L" -> `ExpectingTypeKey`
///    - Type key "SS", "NS", "BS" -> `ExpectingValue`
///    - Type key "S", "N", "B", "BOOL", "NULL" -> `ExpectingValue`
/// - From `ExpectingValue`:
///    - Atom in set -> stays `ExpectingValue`
///    - Otherwise -> error
/// - From `TypeKeyConsumed`:
///    - Field key encountered -> `ExpectingTypeKey`
///
/// End-transitions (when a key or structural element ends):
/// - From `ExpectingValue`:
///    - If parent is "#array" -> `ExpectingTypeKey`
///    - Otherwise -> `TypeKeyConsumed`
/// - From `TypeKeyConsumed`:
///    - If parent is "#array" -> `ExpectingTypeKey`
///    - Otherwise -> `ExpectingField`
/// - From `ExpectingField`:
///    - If key is "Item" at top with `AsWrapper` -> no transition (skipped)
///    - Otherwise -> `TypeKeyConsumed` (M container ended)
/// - From `ExpectingTypeKey`:
///    - Literal type keys (S, N, B, BOOL, NULL) -> `TypeKeyConsumed`
///    - Container type keys (M, L, SS, NS, BS) -> no transition (handled by container end)
///
/// Special container end transitions:
/// - L array ending: `ExpectingTypeKey` -> `ExpectingValue`
/// - SS/NS/BS set ending: `ExpectingValue` -> `ExpectingValue`
/// - M map ending (not in array): -> `ExpectingValue`
/// - M map ending (in array): -> `ExpectingTypeKey`
#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    ExpectingField,
    ExpectingTypeKey,
    ExpectingValue,
    TypeKeyConsumed,
}

/// How to handle "Item" key at top level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemWrapperMode {
    /// Interpret "Item" at top level as a special wrapper
    AsWrapper,
    /// Interpret "Item" at top level as a normal field
    AsField,
}

/// Type descriptor being processed (only for container types)
#[derive(Debug, Clone, Copy, PartialEq)]
enum TypeDesc {
    SS,
    NS, // Sets
    L,
    M, // Nested containers
}

pub struct DdbConverter<'a, 'workbuf, W: IoWrite> {
    writer: &'a mut W,
    pending_comma: bool,
    pretty: bool,
    unbuffered: bool,
    output_depth: usize, // JSON output nesting depth (for pretty-printing indentation and root level detection)
    current_field: Option<&'workbuf [u8]>,
    item_wrapper_mode: ItemWrapperMode, // How to handle "Item" key at top level
    last_error: Option<ConversionError>, // Stores detailed error information (with position)
    last_parse_error_no_pos: Option<ParseErrorNoPos>, // Stores parse error without position (position added later by error handler)

    phase: Phase,
    current_type: Option<TypeDesc>,
}

impl<'a, W: IoWrite> DdbConverter<'a, '_, W> {
    fn new(writer: &'a mut W, pretty: bool, unbuffered: bool, item_wrapper_mode: ItemWrapperMode) -> Self {
        Self {
            writer,
            pending_comma: false,
            pretty,
            unbuffered,
            output_depth: 0,
            current_field: None,
            item_wrapper_mode,
            last_error: None,
            last_parse_error_no_pos: None,
            phase: Phase::ExpectingField,
            current_type: None,
        }
    }

    fn store_rjiter_error(&mut self, error: rjiter::Error, position: usize, context: &'static str) {
        self.last_error = Some(ConversionError::RJiterError {
            kind: error.error_type,
            position,
            context,
        });
    }

    fn store_io_error(
        &mut self,
        kind: embedded_io::ErrorKind,
        position: usize,
        context: &'static str,
    ) {
        self.last_error = Some(ConversionError::IOError {
            kind,
            position,
            context,
        });
    }

    fn store_parse_error(&mut self, context: &'static str, unknown_type_bytes: Option<&[u8]>) {
        let unknown_type = if let Some(bytes) = unknown_type_bytes {
            let len = bytes.len().min(32);
            let mut buffer = [0u8; 32];
            if let (Some(buf_slice), Some(bytes_slice)) = (buffer.get_mut(..len), bytes.get(..len))
            {
                buf_slice.copy_from_slice(bytes_slice);
            }
            Some((buffer, len))
        } else {
            None
        };

        self.last_parse_error_no_pos = Some(ParseErrorNoPos {
            context,
            unknown_type,
        });
    }

    /// Convert stored `ParseErrorNoPos` to `ParseError` with position
    fn finalize_parse_error(&mut self, position: usize) {
        if let Some(parse_error_no_pos) = self.last_parse_error_no_pos.take() {
            self.last_error = Some(parse_error_no_pos.with_position(position));
        }
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), embedded_io::ErrorKind> {
        self.writer.write_all(bytes).map_err(|e| e.kind())?;
        if self.unbuffered {
            self.writer.flush().map_err(|e| e.kind())?;
        }
        Ok(())
    }

    /// Helper that writes and stores error without position
    /// Position will be added later from scan_json's ActionError which calls rjiter.current_index()
    fn try_write_any(&mut self, bytes: &[u8], context: &'static str) -> Result<(), &'static str> {
        // Sentinel value indicating position will be updated later from scan_json
        const POSITION_UPDATED_LATER: usize = usize::MAX;

        self.write(bytes).map_err(|kind| {
            // Store error with sentinel position - scan_json will provide accurate position
            self.store_io_error(kind, POSITION_UPDATED_LATER, context);
            "Write failed"
        })
    }

    fn write_comma_if_pending(&mut self) -> Result<(), &'static str> {
        if self.pending_comma {
            self.try_write_any(b",", "writing comma")?;
            self.newline_if_pretty()?;
            self.pending_comma = false;
        }
        Ok(())
    }

    fn newline_if_pretty(&mut self) -> Result<(), &'static str> {
        if self.pretty {
            self.try_write_any(b"\n", "writing newline")
        } else {
            Ok(())
        }
    }

    fn indent_if_pretty(&mut self) -> Result<(), &'static str> {
        if self.pretty {
            for _ in 0..self.output_depth {
                self.try_write_any(b"  ", "writing indentation")?;
            }
        }
        Ok(())
    }
}

type DdbBaton<'a, 'workbuf, W> = &'a RefCell<DdbConverter<'a, 'workbuf, W>>;

/// Handle root object beginning - write opening brace
fn on_root_object_begin<R: embedded_io::Read, W: IoWrite>(
    _rjiter: &mut RJiter<R>,
    baton: DdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    if let Err(e) = conv.try_write_any(b"{", "writing root object opening brace") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.newline_if_pretty() {
        return StreamOp::Error(e);
    }
    conv.output_depth = 1;
    StreamOp::None
}

/// Handle a field key - write the field name and prepare for type descriptor
fn on_field_key<R: embedded_io::Read, W: IoWrite>(
    _rjiter: &mut RJiter<R>,
    baton: DdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let Some(field_name) = conv.current_field else {
        return StreamOp::Error("Internal error: current_field not set (impossible)");
    };

    if let Err(e) = conv.write_comma_if_pending() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.indent_if_pretty() {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"\"", "writing field name opening quote") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(field_name, "writing field name") {
        return StreamOp::Error(e);
    }
    if let Err(e) = conv.try_write_any(b"\":", "writing field name closing quote and colon") {
        return StreamOp::Error(e);
    }
    conv.pending_comma = false;
    conv.phase = Phase::ExpectingTypeKey;

    StreamOp::None
}

/// Generic helper for writing string-based values (S/B/N types and set elements)
/// Handles peeking, comma writing, quotes, and error reporting
fn write_string_value<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    conv: &mut DdbConverter<'_, '_, W>,
    with_quotes: bool,
    write_comma_if_pending: bool,
    peek_context: &'static str,
    write_context: &'static str,
) -> StreamOp {
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            let position = rjiter.current_index();
            conv.store_rjiter_error(e, position, peek_context);
            return StreamOp::Error("Failed to peek string value");
        }
    };
    if peek != Peek::String {
        return StreamOp::Error("Expected string value");
    }

    if write_comma_if_pending {
        if let Err(e) = conv.write_comma_if_pending() {
            return StreamOp::Error(e);
        }
    }

    if with_quotes {
        if let Err(e) = conv.try_write_any(b"\"", "writing opening quote") {
            return StreamOp::Error(e);
        }
    }
    if let Err(e) = rjiter.write_long_bytes(conv.writer) {
        let position = rjiter.current_index();
        conv.store_rjiter_error(e, position, write_context);
        return StreamOp::Error("Failed to write value");
    }
    if conv.unbuffered {
        if let Err(e) = conv.writer.flush() {
            let position = rjiter.current_index();
            conv.store_io_error(e.kind(), position, "flushing after write_long_bytes");
            return StreamOp::Error("Failed to flush writer");
        }
    }
    if with_quotes {
        if let Err(e) = conv.try_write_any(b"\"", "writing closing quote") {
            return StreamOp::Error(e);
        }
    }

    conv.pending_comma = true;
    StreamOp::ValueIsConsumed
}

/// Helper for boolean-based types (BOOL/NULL): peek bool, consume with `known_bool`, write output
fn handle_bool_based_type<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    conv: &mut DdbConverter<'_, '_, W>,
    validate_peek: impl Fn(Peek) -> Result<&'static [u8], &'static str>,
    type_name: &'static str,
) -> StreamOp {
    let peek = match rjiter.peek() {
        Ok(p) => p,
        Err(e) => {
            let position = rjiter.current_index();
            conv.store_rjiter_error(e, position, type_name);
            return StreamOp::Error("Failed to peek value");
        }
    };

    // Validate and get output bytes
    let output = match validate_peek(peek) {
        Ok(bytes) => bytes,
        Err(msg) => return StreamOp::Error(msg),
    };

    // Write comma (pending_comma tracks whether we need it)
    if let Err(e) = conv.write_comma_if_pending() {
        return StreamOp::Error(e);
    }

    // Consume the value
    if let Err(e) = rjiter.known_bool(peek) {
        let position = rjiter.current_index();
        conv.store_rjiter_error(e, position, type_name);
        return StreamOp::Error("Failed to consume boolean value");
    }
    if let Err(e) = conv.try_write_any(output, type_name) {
        return StreamOp::Error(e);
    }

    conv.pending_comma = true;
    conv.current_type = None;
    StreamOp::ValueIsConsumed
}

/// Handle a type key - for literal types, consume and write the value directly
fn on_type_key<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: DdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    let Some(type_key) = conv.current_field else {
        return StreamOp::Error("current_field should be set for type key");
    };

    match type_key {
        b"S" | b"B" => {
            let result = write_string_value(
                rjiter,
                &mut conv,
                true,
                true,
                "S/B (string) type",
                "S/B (string) type",
            );
            conv.current_type = None;
            conv.phase = Phase::ExpectingValue;
            result
        }
        b"N" => {
            let result = write_string_value(
                rjiter,
                &mut conv,
                false,
                true,
                "N (number) type",
                "N (number) type",
            );
            conv.current_type = None;
            conv.phase = Phase::ExpectingValue;
            result
        }
        b"BOOL" => {
            let result = handle_bool_based_type(
                rjiter,
                &mut conv,
                |peek| match peek {
                    Peek::True => Ok(b"true"),
                    Peek::False => Ok(b"false"),
                    _ => Err("Expected boolean value for BOOL type"),
                },
                "BOOL type",
            );
            conv.phase = Phase::ExpectingValue;
            result
        }
        b"NULL" => {
            let result = handle_bool_based_type(
                rjiter,
                &mut conv,
                |peek| match peek {
                    Peek::True => Ok(b"null"),
                    _ => Err("Expected true for NULL type"),
                },
                "NULL type",
            );
            conv.phase = Phase::ExpectingValue;
            result
        }
        b"SS" | b"BS" => {
            // SS/BS type - write opening bracket here (parent handles it, not find_action_array)
            if let Err(e) = conv.try_write_any(b"[", "writing SS/BS opening bracket") {
                return StreamOp::Error(e);
            }
            conv.pending_comma = false;
            conv.current_type = Some(TypeDesc::SS);
            conv.phase = Phase::ExpectingValue; // Stay in ExpectingValue, SS elements are atoms
            StreamOp::None
        }
        b"NS" => {
            // NS type - write opening bracket here (parent handles it, not find_action_array)
            if let Err(e) = conv.try_write_any(b"[", "writing NS opening bracket") {
                return StreamOp::Error(e);
            }
            conv.pending_comma = false;
            conv.current_type = Some(TypeDesc::NS);
            conv.phase = Phase::ExpectingValue; // Stay in ExpectingValue, NS elements are atoms
            StreamOp::None
        }
        b"L" => {
            // L type - write opening bracket here (parent handles it, not find_action_array)
            if let Err(e) = conv.write_comma_if_pending() {
                return StreamOp::Error(e);
            }
            if let Err(e) = conv.try_write_any(b"[", "writing L opening bracket") {
                return StreamOp::Error(e);
            }
            conv.pending_comma = false;
            conv.current_type = Some(TypeDesc::L);
            conv.phase = Phase::ExpectingTypeKey; // In L, we expect type keys (type descriptors are ignored)
            StreamOp::None
        }
        b"M" => {
            // M type - write opening brace here (parent handles it, not find_action_object)
            if let Err(e) = conv.write_comma_if_pending() {
                return StreamOp::Error(e);
            }
            if let Err(e) = conv.try_write_any(b"{", "writing M opening brace") {
                return StreamOp::Error(e);
            }
            if let Err(e) = conv.newline_if_pretty() {
                return StreamOp::Error(e);
            }
            conv.output_depth += 1;
            conv.pending_comma = false;
            conv.current_type = Some(TypeDesc::M);
            conv.phase = Phase::ExpectingField;
            StreamOp::None
        }
        _ => {
            conv.store_parse_error(
                "Invalid DynamoDB JSON format: unknown type descriptor",
                Some(type_key),
            );
            conv.finalize_parse_error(rjiter.current_index());
            StreamOp::Error("Unknown type descriptor")
        }
    }
}

// Type descriptor value handlers for set element atoms (SS, NS)

fn on_set_string_element<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: DdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    write_string_value(
        rjiter,
        &mut conv,
        true, // with_quotes
        true, // write_comma_if_pending: always for set elements (pending_comma handles first element)
        "peeking SS/BS (string set) element",
        "writing SS/BS (string set) element",
    )
}

fn on_set_number_element<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: DdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    write_string_value(
        rjiter,
        &mut conv,
        false, // with_quotes
        true, // write_comma_if_pending: always for set elements (pending_comma handles first element)
        "peeking NS (number set) element",
        "writing NS (number set) element",
    )
}

// Generic error handler that converts ParseErrorNoPos to ParseError with position
fn on_error<R: embedded_io::Read, W: IoWrite>(
    rjiter: &mut RJiter<R>,
    baton: DdbBaton<'_, '_, W>,
) -> StreamOp {
    let mut conv = baton.borrow_mut();
    conv.finalize_parse_error(rjiter.current_index());
    StreamOp::Error("Validation error (see stored error)")
}
/// Handle Object structural pseudoname
fn find_action_object<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    mut context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
    current_type: Option<TypeDesc>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    if baton.borrow().output_depth == 0 {
        return Some(on_root_object_begin);
    }

    match phase {
        Phase::ExpectingValue => {
            // In ExpectingValue, only M type expects objects; all others (SS, NS, L) expect arrays
            // If we're here with an object, it's invalid
            let mut conv = baton.borrow_mut();
            conv.store_parse_error(
                "Invalid DynamoDB JSON format: unexpected object value",
                None,
            );
            Some(on_error)
        }
        Phase::ExpectingTypeKey => {
            // Check if L type is expecting an array but got an object
            if current_type == Some(TypeDesc::L) {
                let mut conv = baton.borrow_mut();
                conv.store_parse_error(
                    "Invalid DynamoDB JSON format: L type expects an array value, not an object",
                    None,
                );
                return Some(on_error);
            }
            // Type descriptor objects are allowed in other contexts
            None
        }
        Phase::ExpectingField => {
            // M type context - objects (nested M values) are allowed
            None
        }
        Phase::TypeKeyConsumed => {
            // In array context, TypeKeyConsumed allows new type descriptor objects
            // Check if we're in an array
            let parent = context.next();

            if parent == Some(b"#array") {
                // In array - allow type descriptor objects
                None
            } else {
                // Not in array - error
                let mut conv = baton.borrow_mut();
                conv.store_parse_error(
                    "Invalid DynamoDB JSON format: unexpected nested object in type descriptor",
                    None,
                );
                Some(on_error)
            }
        }
    }
}

fn find_action_key<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    mut context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    let key = context.next()?;

    // Begin-transitions (based on current phase before processing the key)
    match phase {
        Phase::ExpectingField => {
            // Check for Item at top with AsWrapper - early return without side effects
            if key == b"Item" {
                let mode = baton.borrow().item_wrapper_mode;
                if mode == ItemWrapperMode::AsWrapper {
                    if let Some(b"#top") = context.next() {
                        return None;
                    }
                }
            }
            // Store the key
            let mut conv = baton.borrow_mut();
            #[allow(unsafe_code)]
            let key_slice: &'workbuf [u8] =
                unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(key) };
            conv.current_field = Some(key_slice);
            // Transition: ExpectingField -> ExpectingTypeKey
            // (This transition is handled by on_field_key which sets phase to ExpectingTypeKey)
            Some(on_field_key)
        }
        Phase::ExpectingTypeKey => {
            // Store the key
            let mut conv = baton.borrow_mut();
            #[allow(unsafe_code)]
            let key_slice: &'workbuf [u8] =
                unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(key) };
            conv.current_field = Some(key_slice);

            // Transition: ExpectingTypeKey -> if in "M", then ExpectingField; otherwise, ExpectingValue
            // Note: The actual transition happens in on_type_key based on the type
            Some(on_type_key)
        }
        Phase::ExpectingValue => {
            // Transition: ExpectingValue -> if in a set, ExpectingValue; otherwise, error
            let current_type = baton.borrow().current_type;

            // Check if we're in a set (SS, NS)
            let in_set = matches!(current_type, Some(TypeDesc::SS | TypeDesc::NS));

            if !in_set {
                // Error: not in a set
                let mut conv = baton.borrow_mut();
                conv.store_parse_error("Unexpected key in ExpectingValue phase (not in set)", None);
                return Some(on_error);
            }

            // Otherwise, continue in ExpectingValue phase
            // Store the key
            let mut conv = baton.borrow_mut();
            #[allow(unsafe_code)]
            let key_slice: &'workbuf [u8] =
                unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(key) };
            conv.current_field = Some(key_slice);

            // The phase remains ExpectingValue (handled by the type handler)
            Some(on_type_key)
        }
        Phase::TypeKeyConsumed => {
            // Transition: TypeKeyConsumed -> ExpectingField (must be in M object)
            // Store the key
            let mut conv = baton.borrow_mut();
            #[allow(unsafe_code)]
            let key_slice: &'workbuf [u8] =
                unsafe { core::mem::transmute::<&[u8], &'workbuf [u8]>(key) };
            conv.current_field = Some(key_slice);
            // Transition happens through on_field_key
            Some(on_field_key)
        }
    }
}

/// Handle Array structural pseudoname
fn find_action_array<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    _context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    _phase: Phase,
    current_type: Option<TypeDesc>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Validate context: only allow arrays for SS, NS, L types
    match current_type {
        Some(TypeDesc::SS | TypeDesc::NS) => {
            // Valid: these types expect arrays
            None
        }
        Some(TypeDesc::L) => {
            // Valid: L expects array. Clear current_type so elements inside don't inherit it
            let mut conv = baton.borrow_mut();
            conv.current_type = None;
            None
        }
        _ => {
            // All other cases: arrays are not valid
            let mut conv = baton.borrow_mut();
            conv.store_parse_error("Invalid DynamoDB JSON format: unexpected array value", None);
            Some(on_error)
        }
    }
}

/// Handle Atom structural pseudoname
#[allow(clippy::unnecessary_wraps)]
fn find_action_atom<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    mut context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
    current_type: Option<TypeDesc>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    // Atoms are only valid as set elements inside arrays
    if phase != Phase::ExpectingValue {
        let mut conv = baton.borrow_mut();
        conv.store_parse_error(
            "Invalid DynamoDB JSON format: Expected array for set type, atom values only allowed as set elements",
            None,
        );
        return Some(on_error);
    }

    // SS/NS set element inside array
    if matches!(current_type, Some(TypeDesc::SS | TypeDesc::NS)) {
        if let Some(first) = context.next() {
            if first == b"#array" {
                // Valid set element inside array
                return if current_type == Some(TypeDesc::SS) {
                    Some(on_set_string_element)
                } else {
                    Some(on_set_number_element)
                };
            }
        }
    }

    // All other cases: atoms are unexpected
    let mut conv = baton.borrow_mut();
    conv.store_parse_error(
        "Invalid DynamoDB JSON format: Expected array for set type, atom values only allowed as set elements",
        None,
    );
    Some(on_error)
}

fn find_action<'a, 'workbuf, R: embedded_io::Read, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<Action<DdbBaton<'a, 'workbuf, W>, R>> {
    let (phase, current_type) = {
        let conv = baton.borrow();
        (conv.phase, conv.current_type)
    };

    // Match on structural type and delegate to appropriate handler
    match structural {
        StructuralPseudoname::Object => find_action_object(context, baton, phase, current_type),
        StructuralPseudoname::None => find_action_key(context, baton, phase),
        StructuralPseudoname::Array => find_action_array(context, baton, phase, current_type),
        StructuralPseudoname::Atom => find_action_atom(context, baton, phase, current_type),
    }
}

fn on_list_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();

    // Validate: ending an array is only allowed in ExpectingTypeKey phase
    if conv.phase != Phase::ExpectingTypeKey {
        return Err("Invalid phase when ending L array (expected ExpectingTypeKey)");
    }

    conv.try_write_any(b"]", "writing L closing bracket")?;
    conv.pending_comma = true;

    // Transition: ExpectingTypeKey -> ExpectingValue (at end of array)
    conv.phase = Phase::ExpectingValue;
    conv.current_type = None;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_set_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.try_write_any(b"]", "writing SS/NS/BS closing bracket")?;
    conv.pending_comma = true;

    // Ending SS/NS set - transition to ExpectingValue
    conv.current_type = None;
    conv.phase = Phase::ExpectingValue;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_map_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline_if_pretty()?;
    conv.output_depth -= 1;
    conv.indent_if_pretty()?;
    conv.try_write_any(b"}", "writing M closing brace")?;
    conv.pending_comma = true;

    // M container value is consumed
    conv.current_type = None;
    conv.phase = Phase::TypeKeyConsumed;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_map_end_in_array<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    on_map_end(baton)?;
    // Override phase to ExpectingTypeKey for array context
    let mut conv = baton.borrow_mut();
    conv.phase = Phase::ExpectingTypeKey;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_type_key_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Called for the phase "TypeKeyConsumed"

    // Type key value ended (for literal types: S, N, B, BOOL, NULL)
    // Transition to ExpectingField
    let mut conv = baton.borrow_mut();
    conv.phase = Phase::ExpectingField;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_type_key_end_in_array<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    // Called for the phase "TypeKeyConsumed" when in an array context

    // Type key value ended in array - transition to ExpectingTypeKey
    let mut conv = baton.borrow_mut();
    conv.phase = Phase::ExpectingTypeKey;
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn on_root_object_end<W: IoWrite>(baton: DdbBaton<'_, '_, W>) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.newline_if_pretty()?;
    conv.try_write_any(b"}", "writing root object closing brace")?;
    conv.try_write_any(b"\n", "writing final newline")?;

    // Reset state for next JSONL record
    conv.pending_comma = false;
    conv.output_depth = 0;
    conv.phase = Phase::ExpectingField;

    Ok(())
}

/// Handle Object structural pseudoname for end actions
fn find_end_action_object<'a, 'workbuf, W: IoWrite>(
    context: &ContextIter,
    _baton: DdbBaton<'a, 'workbuf, W>,
    _phase: Phase,
) -> Option<EndAction<DdbBaton<'a, 'workbuf, W>>> {
    if context.len() == 1 {
        return Some(on_root_object_end);
    }
    None
}

/// Transition to `TypeKeyConsumed` phase
#[allow(clippy::unnecessary_wraps)]
fn on_transition_to_type_key_consumed<W: IoWrite>(
    baton: DdbBaton<'_, '_, W>,
) -> Result<(), &'static str> {
    let mut conv = baton.borrow_mut();
    conv.phase = Phase::TypeKeyConsumed;
    Ok(())
}

/// Handle end-actions for keys - this is where all end-action logic resides
fn find_end_action_key<'a, 'workbuf, W: IoWrite>(
    mut context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
    phase: Phase,
) -> Option<EndAction<DdbBaton<'a, 'workbuf, W>>> {
    let key = context.next()?;

    // End-transitions (based on current phase when the key ends)
    match phase {
        Phase::ExpectingValue => {
            // Transition: ExpectingValue -> TypeKeyConsumed
            // But if we're in an array context, transition to ExpectingTypeKey instead
            let parent = context.next();

            if parent == Some(b"#array") {
                Some(on_type_key_end_in_array)
            } else {
                Some(on_transition_to_type_key_consumed)
            }
        }
        Phase::TypeKeyConsumed => {
            // Transition: TypeKeyConsumed -> if in "#array", then ExpectingTypeKey; otherwise, ExpectingField
            // Check context
            let parent = context.next();

            if parent == Some(b"#array") {
                Some(on_type_key_end_in_array)
            } else {
                Some(on_type_key_end)
            }
        }
        Phase::ExpectingField => {
            // Check for Item at top with AsWrapper - early return without side effects
            if key == b"Item" {
                let mode = baton.borrow().item_wrapper_mode;
                if let Some(b"#top") = context.next() {
                    if mode == ItemWrapperMode::AsWrapper {
                        return None;
                    }
                }
            }

            // ExpectingField phase only occurs inside M containers or at root level
            // When a field ends in ExpectingField, it means the M container is ending
            // Check parent context to determine the appropriate transition
            let parent = context.next();

            if parent == Some(b"#array") {
                // M container ending inside an L array - write "}" and transition to ExpectingTypeKey
                Some(on_map_end_in_array)
            } else {
                // M container ending normally - write "}" and transition to ExpectingValue
                Some(on_map_end)
            }
        }
        Phase::ExpectingTypeKey => {
            // For type keys ending in ExpectingTypeKey:
            // - Container types (M, L, SS, NS, BS) are handled by their specific end handlers (return None)
            // - Literal types (S, N, BOOL, NULL, B) transition to TypeKeyConsumed
            let is_container_type = matches!(key, b"M" | b"L" | b"SS" | b"NS" | b"BS");
            if is_container_type {
                None // Container end is handled elsewhere
            } else {
                // Literal type - transition to TypeKeyConsumed
                Some(on_transition_to_type_key_consumed)
            }
        }
    }
}

fn find_end_action<'a, 'workbuf, W: IoWrite>(
    structural: StructuralPseudoname,
    context: ContextIter,
    baton: DdbBaton<'a, 'workbuf, W>,
) -> Option<EndAction<DdbBaton<'a, 'workbuf, W>>> {
    let (phase, _current_type) = {
        let conv = baton.borrow();
        (conv.phase, conv.current_type)
    };

    match structural {
        StructuralPseudoname::Array => {
            // Check if we're ending an L array or a set (SS, NS, BS)
            // L arrays end in ExpectingTypeKey phase (since they contain type descriptors)
            // Sets end in ExpectingValue phase (since they contain raw values)
            if phase == Phase::ExpectingTypeKey {
                Some(on_list_end)
            } else if phase == Phase::ExpectingValue {
                Some(on_set_end)
            } else {
                None
            }
        }
        StructuralPseudoname::Object => find_end_action_object(&context, baton, phase),
        StructuralPseudoname::None => find_end_action_key(context, baton, phase),
        StructuralPseudoname::Atom => None,
    }
}

/// Convert `DynamoDB` JSON to normal JSON in a streaming, allocation-free manner.
/// Supports JSONL format (newline-delimited JSON) - processes multiple JSON objects.
///
/// # Arguments
/// * `reader` - Input stream implementing `embedded_io::Read`
/// * `writer` - Output stream implementing `embedded_io::Write`
/// * `rjiter_buffer` - Buffer for rjiter to use (recommended: 4096 bytes)
/// * `context_buffer` - Buffer for `scan_json` context tracking (recommended: 2048 bytes)
/// * `pretty` - Whether to pretty-print the output
/// * `unbuffered` - Whether to flush after every write
/// * `item_wrapper_mode` - How to handle "Item" key at top level (`AsWrapper` or `AsField`)
///
/// # Errors
/// Returns `ConversionError` if:
/// - Input JSON is malformed or invalid
/// - Input contains invalid `DynamoDB` type descriptors
/// - I/O errors occur during reading or writing
/// - Buffer sizes are insufficient for the input data
///
/// # Returns
/// `Ok(())` on success, or `Err(ConversionError)` with detailed error information on failure
pub fn convert_ddb_to_normal<R: IoRead, W: IoWrite>(
    reader: &mut R,
    writer: &mut W,
    rjiter_buffer: &mut [u8],
    context_buffer: &mut [u8],
    pretty: bool,
    unbuffered: bool,
    item_wrapper_mode: ItemWrapperMode,
) -> Result<(), ConversionError> {
    let mut rjiter = RJiter::new(reader, rjiter_buffer);

    let converter = DdbConverter::new(writer, pretty, unbuffered, item_wrapper_mode);
    let baton = RefCell::new(converter);

    // DynamoDB supports up to 32 levels of nesting in the original data.
    // In DynamoDB JSON format, each nested object/array adds extra levels:
    // - Each Map: {"M": {...}} adds 1 level
    // - Each List: {"L": [...]} adds 1 level
    // - Optional "Item" wrapper adds 1 level
    // For 32 levels: 1 (Item/#top) + 32 (level_N) + 32 (M) + 1 (value) + 1 (S) + 1 (leaf value) = 68 slots
    let mut context = U8Pool::new(context_buffer, 68).map_err(|_| {
        ConversionError::ScanError(scan_json::Error::InternalError {
            position: 0,
            message: "Failed to create context pool",
        })
    })?;

    if let Err(e) = scan(
        find_action,
        find_end_action,
        &mut rjiter,
        &baton,
        &mut context,
        &Options::new(),
    ) {
        // Check if there's a stored detailed error in the baton
        let stored_error = baton.borrow_mut().last_error.take();
        if let Some(mut err) = stored_error {
            // Extract position from scan_json's error and update our stored error
            // scan_json calls rjiter.current_index() which gives accurate position
            let position = match &e {
                scan_json::Error::ActionError { position, .. } => *position,
                scan_json::Error::MaxNestingExceeded { position, .. } => *position,
                scan_json::Error::InternalError { position, .. } => *position,
                scan_json::Error::UnhandledPeek { position, .. } => *position,
                scan_json::Error::UnbalancedJson(position) => *position,
                scan_json::Error::RJiterError(e) => e.index,
                scan_json::Error::IOError(_) => rjiter.current_index(), // Get position from rjiter
            };
            // Update position in stored error (replaces POSITION_UPDATED_LATER sentinel)
            match &mut err {
                ConversionError::IOError { position: p, .. } => *p = position,
                ConversionError::RJiterError { position: p, .. } => *p = position,
                ConversionError::ParseError { position: p, .. } => *p = position,
                ConversionError::ScanError(_) => {}
            }
            return Err(err);
        }
        // Otherwise return the scan error
        return Err(ConversionError::ScanError(e));
    }

    Ok(())
}
