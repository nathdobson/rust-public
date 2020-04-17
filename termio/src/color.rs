#[derive(Eq, PartialOrd, PartialEq, Ord, Hash, Copy, Clone, Debug)]
pub enum BaseColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
}

#[derive(Eq, PartialOrd, PartialEq, Ord, Hash, Copy, Clone, Debug)]
pub enum Color {
    Dark(BaseColor),
    Bright(BaseColor),
    RGB666(u8, u8, u8),
    Gray24(u8),
}

impl Color {
    pub fn into_u8(self) -> u8 {
        match self {
            Color::Dark(c) => c as u8,
            Color::Bright(c) => c as u8 + 8,
            Color::RGB666(r, g, b) => {
                assert!(r < 6 && g < 6 && b < 6);
                16 + r * 36 + g * 6 + b
            }
            Color::Gray24(n) => {
                assert!(n < 24);
                232 + n
            }
        }
    }
}