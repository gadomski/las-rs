/// A RGB color value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Red channel.
    pub red: u16,
    /// Green channel.
    pub green: u16,
    /// Blue channel.
    pub blue: u16,
}
