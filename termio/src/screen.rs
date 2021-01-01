use util::grid::Grid;
use crate::color::Color;
use util::rangemap::RangeMap;
use crate::writer::TermWriter;
use util::grid;
use std::collections::{BTreeSet, BTreeMap};
use std::fmt;
use crate::output::*;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Row {
    pub runes: RangeMap<isize, Rune>,
    pub line_setting: LineSetting,
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct Screen {
    pub title: String,
    pub rows: BTreeMap<isize, Row>,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Rune {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Default, Debug)]
pub struct __UseDefaultDefaultToBuildStyle;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Default)]
pub struct Style {
    pub background: Color,
    pub foreground: Color,
    #[doc(hidden)]
    pub __use_default_default_to_build_style__: __UseDefaultDefaultToBuildStyle,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub enum LineSetting {
    Normal,
    DoubleHeightTop,
    DoubleHeightBottom,
}

impl LineSetting {
    pub fn merge(self, other: Self) -> Self {
        if self == other {
            self
        } else {
            eprintln!("line setting mismatch {:?} {:?}", self, other);
            LineSetting::Normal
        }
    }
}

impl Rune {
    fn new() -> Self {
        Rune {
            text: " ".to_string(),
            style: Default::default(),
        }
    }
}

impl Default for LineSetting {
    fn default() -> Self {
        LineSetting::Normal
    }
}

impl Style {
    fn new() -> Self {
        Style::default()
    }
}

pub fn advance(string: &str) -> isize {
    match string {
        "" => 0,
        "Ａ" | "Ｋ" | "Ｑ" | "Ｊ" | "🐕" | "🦅" | "🐉" => 2,
        _ => 1,
    }
}


impl Screen {
    pub fn new() -> Self {
        Screen {
            title: "".to_string(),
            rows: BTreeMap::new(),
        }
    }
    pub fn clear(&mut self) {
        self.rows.clear();
    }
    pub fn row(&mut self, y: isize) -> &mut Row {
        self.rows.entry(y).or_insert(Row::new())
    }
    pub fn title(&mut self) -> &mut String {
        &mut self.title
    }
}

impl Row {
    pub fn new() -> Self {
        Row { runes: RangeMap::new(), line_setting: Default::default() }
    }
}

impl fmt::Debug for Rune {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}{}",
               Foreground(self.style.foreground),
               Background(self.style.background),
               self.text,
               NoFormat)
    }
}

impl fmt::Debug for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Style")
            .field("background", &self.background)
            .field("foreground", &self.foreground)
            .finish()
    }
}

impl fmt::Debug for Screen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (y, row) in self.rows.iter() {
            write!(f, "{:?} {:?} ", y, row.line_setting)?;
            for x in row.runes.iter() {
                write!(f, "{:?}", x)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
