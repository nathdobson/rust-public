use unicode_segmentation::UnicodeSegmentation;

use crate::string::{StyleFormat, StyleString};

pub fn advance_of_grapheme(string: &str) -> isize {
    match string {
        "" => 0,
        "Ａ" | "Ｋ" | "Ｑ" | "Ｊ" | "🐕" | "🦅" | "🐉" => 2,
        _ => 1,
    }
}

pub fn advance_of_string(string: &str) -> isize {
    string.graphemes(true).map(advance_of_grapheme).sum()
}

pub fn advance_of_style_string(ss: &StyleString) -> isize {
    ss.inner().iter().map(|(_, s)| advance_of_string(s)).sum()
}
