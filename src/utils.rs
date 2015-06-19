//! Private utility functions.

use Error;
use Result;

/// Converts a `u8` buffer to a string, using null bytes as the terminator.
///
/// Also checks for characters after the first null byte, which is invalid according to the las
/// spec.
pub fn buffer_to_string(buffer: &[u8]) -> Result<String> {
    let mut s = String::with_capacity(buffer.len());
    let mut seen_null_byte = false;
    for &byte in buffer {
        if byte > 0u8 {
            if seen_null_byte {
                return Err(Error::CharacterAfterNullByte);
            } else {
                s.push(byte as char);
            }
        } else {
            seen_null_byte = true;
        }
    }
    Ok(s)
}

