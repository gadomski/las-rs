const MASK: u8 = 0b00111000;

/// The total number of returns detected from a given pulse.
#[derive(Clone, Copy, Debug, Default)]
pub struct NumberOfReturns(u8);

impl NumberOfReturns {
    /// True if this return number is valid (between one and five).
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::NumberOfReturns;
    /// assert!(NumberOfReturns::from(8).is_valid());
    /// assert!(!NumberOfReturns::from(0b00111000).is_valid());
    /// ```
    pub fn is_valid(&self) -> bool {
        self.0 > 0 && self.0 < 6
    }
}

impl From<u8> for NumberOfReturns {
    fn from(n: u8) -> NumberOfReturns {
        NumberOfReturns((n & MASK) >> 3)
    }
}

impl From<NumberOfReturns> for u8 {
    fn from(number_of_returns: NumberOfReturns) -> u8 {
        number_of_returns.0 << 3
    }
}

impl PartialEq<NumberOfReturns> for u8 {
    fn eq(&self, other: &NumberOfReturns) -> bool {
        *self == other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_of_returns_u8() {
        assert_eq!(1, NumberOfReturns::from(0b00001000));
        assert_eq!(7, NumberOfReturns::from(0b00111000));
        assert_eq!(1, NumberOfReturns::from(0b01001111));
        assert_eq!(0b00001000, u8::from(NumberOfReturns(1)));
        assert_eq!(0b00111000, u8::from(NumberOfReturns(7)));
    }

    #[test]
    fn number_of_returns_is_valid() {
        assert!(!NumberOfReturns::from(0).is_valid());
        assert!(NumberOfReturns::from(8).is_valid());
        assert!(NumberOfReturns::from(0b00101000).is_valid());
        assert!(!NumberOfReturns::from(0b00111000).is_valid());
    }
}
