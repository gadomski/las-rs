/// A RGB color value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    /// Red channel.
    pub red: u16,

    /// Green channel.
    pub green: u16,

    /// Blue channel.
    pub blue: u16,
}

impl Color {
    /// Creates a new color.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Color;
    /// let color = Color::new(1, 2, 3);
    /// assert_eq!(1, color.red);
    /// assert_eq!(2, color.green);
    /// assert_eq!(3, color.blue);
    /// ```
    pub fn new(red: u16, green: u16, blue: u16) -> Color {
        Color { red, green, blue }
    }
}
