//! Utilities for reading and writing

use std::io;

use byteorder::{Error, Result};

/// Reads enough data to fill the buffer, or errors.
///
/// This is taken from the byteorder souce, where it is a private method.
pub fn read_full<R: io::Read + ?Sized>(rdr: &mut R, buf: &mut [u8]) -> Result<()> {
    let mut nread = 0usize;
    while nread < buf.len() {
        match rdr.read(&mut buf[nread..]) {
            Ok(0) => return Err(Error::UnexpectedEOF),
            Ok(n) => nread += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(From::from(e)),
        }
    }
    Ok(())
}

/// Writes a certain number of null bytes to a buffer.
///
/// This is basically `write_all` that returns the number of bytes written.
pub fn write_zeros<W: io::Write>(writer: &mut W, n: usize) -> io::Result<usize> {
    let buf = vec![0; n];
    try!(writer.write_all(&buf[..]));
    Ok(n)
}
