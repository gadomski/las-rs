//! I/O related extensions.

use std::io::Read;

use Error;
use Result;

pub trait LasStringExt: Read {
    fn read_las_string(&mut self, count: usize) -> Result<String>;
}

impl<R: Read> LasStringExt for R {
    fn read_las_string(&mut self, count: usize) -> Result<String> {
        let mut character_after_null = false;
        let mut seen_null = false;
        let mut string = String::with_capacity(count);
        for byte in self.take(count as u64).bytes() {
            let byte = try!(byte);
            if byte == 0u8 {
                seen_null = true;
            } else if seen_null {
                character_after_null = true;
            } else {
                string.push(byte as char);
            }
        }
        if character_after_null {
            Err(Error::CharacterAfterNullByte)
        } else {
            Ok(string)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    #[test]
    fn simple_las_string() {
        let mut cursor = Cursor::new(Vec::from("hi"));
        assert_eq!("hi", cursor.read_las_string(2).unwrap());
    }
}
