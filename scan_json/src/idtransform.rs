use crate::{rjiter::jiter::Peek, Error as ScanError, RJiter, Result as ScanResult};
use std::io::Write;

/// Copy a JSON atom (string, number, boolean, or null) from the input to the output.
/// Advances the input iterator to the next token.
///
/// # Errors
///
/// This function will return an error if:
/// * The input JSON is malformed
/// * An IO error occurs while writing to the output
/// * An unexpected token type is encountered
pub fn copy_atom(peeked: Peek, rjiter: &mut RJiter, writer: &mut dyn Write) -> ScanResult<()> {
    if peeked == Peek::String {
        rjiter.write_long_bytes(writer)?;
        return Ok(());
    }
    if peeked == Peek::Null {
        rjiter.known_null()?;
        writer.write_all(b"null")?;
        return Ok(());
    }
    if peeked == Peek::True {
        rjiter.known_bool(peeked)?;
        writer.write_all(b"true")?;
        return Ok(());
    }
    if peeked == Peek::False {
        rjiter.known_bool(peeked)?;
        writer.write_all(b"false")?;
        return Ok(());
    }
    let maybe_number = rjiter.next_number_bytes();
    if let Ok(number) = maybe_number {
        writer.write_all(number)?;
        return Ok(());
    };
    Err(ScanError::UnhandledPeek(peeked, rjiter.current_index()))
}

pub fn idtransform(_rjiter: &mut RJiter, _writer: &mut dyn Write) {
    #[warn(clippy::needless_return)]
    return;
}
