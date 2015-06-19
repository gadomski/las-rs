//! Private utility functions.

use std::io::Read;

use Error;
use Result;

/// Reads `n` bytes from a `Read` and creates a string from the result.
///
/// All nulls are interpreted as unused bytes. A non-null byte after a null byte is considered an
/// error.
pub fn read_into_string<R: Read>(reader: &mut R, count: usize) -> Result<String> {
    let mut s = String::with_capacity(count);
    let mut buffer = Vec::with_capacity(count);
    let mut seen_null_byte = false;
    if try!(reader.take(count as u64).read_to_end(&mut buffer)) != count {
        return Err(Error::ReadError(format!("Read error when reading {} bytes into string", count)));
    }
    for &byte in &buffer {
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    #[test]
    fn read_into_string_ok() {
        let ref mut cursor = Cursor::new(Vec::from("hi"));
        let result = read_into_string(cursor, 2);
        assert!(result.is_ok());
        assert_eq!("hi", result.unwrap());
    }
}
