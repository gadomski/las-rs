//! I/O related extensions.

use std::io::Read;

use Result;

pub trait LasStringExt: Read {
    fn read_las_string(&mut self, count: usize) -> Result<String>;
}

impl<R: Read> LasStringExt for R {
    fn read_las_string(&mut self, count: usize) -> Result<String> {
        Ok(String::new())
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
