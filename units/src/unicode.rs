use std::fmt::{Debug, Formatter};

pub struct Superscript(pub i32);

impl Debug for Superscript {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == 1 {
            return Ok(());
        }
        write!(
            f,
            "{}",
            format!("{}", self.0)
                .chars()
                .map(|c| match c {
                    '-' => "⁻",
                    '0' => "⁰",
                    '1' => "¹",
                    '2' => "²",
                    '3' => "³",
                    '4' => "⁴",
                    '5' => "⁵",
                    '6' => "⁶",
                    '7' => "⁷",
                    '8' => "⁸",
                    '9' => "⁹",
                    _ => unreachable!(),
                })
                .collect::<String>()
        )
    }
}
