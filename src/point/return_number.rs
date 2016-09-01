/// An output laser can have many returns, and each return must be marked in sequence.
///
/// The LAS format limits the number of returns that can be respresented, hense the newtype.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReturnNumber(u8);

impl ReturnNumber {
    /// True if this return number is valid (between one and five).
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::ReturnNumber;
    /// assert!(ReturnNumber::from(1).is_valid());
    /// assert!(!ReturnNumber::from(6).is_valid());
    /// ```
    pub fn is_valid(&self) -> bool {
        self.0 > 0 && self.0 < 6
    }
}

impl From<u8> for ReturnNumber {
    fn from(n: u8) -> ReturnNumber {
        ReturnNumber(n & 7)
    }
}

impl From<ReturnNumber> for u8 {
    fn from(return_number: ReturnNumber) -> u8 {
        return_number.0
    }
}

impl PartialEq<ReturnNumber> for u8 {
    fn eq(&self, other: &ReturnNumber) -> bool {
        *self == other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_number_from_u8() {
        assert_eq!(1, ReturnNumber::from(1));
        assert_eq!(7, ReturnNumber::from(7));
        assert_eq!(1, ReturnNumber::from(9));
        assert_eq!(1, u8::from(ReturnNumber(1)));
        assert_eq!(7, u8::from(ReturnNumber(7)));
    }

    #[test]
    fn return_number_valid() {
        assert!(!ReturnNumber::from(0).is_valid());
        assert!(ReturnNumber::from(1).is_valid());
        assert!(ReturnNumber::from(5).is_valid());
        assert!(!ReturnNumber::from(7).is_valid());
    }
}
