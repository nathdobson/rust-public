use util::grid::Grid;
use crate::color::Color;
use util::rangemap::RangeMap;
use crate::writer::TermWriter;
use util::grid;
use std::collections::{BTreeSet, BTreeMap};
use std::fmt;
use crate::output::*;
use std::fmt::Debug;
use arrayvec::ArrayString;
use vec_map::VecMap;
use std::mem::size_of;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct Row {
    pub runes: Vec<Rune>,
    pub line_setting: LineSetting,
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct Screen {
    pub title: String,
    pub rows: Vec<Row>,
    pub background: Style,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Rune {
    pub text: ArrayString<7>,
    pub style: Style,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Default, Debug)]
pub struct __UseDefaultDefaultToBuildStyle;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Default)]
pub struct Style {
    pub background: Color,
    pub foreground: Color,
    pub bold: bool,
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
    pub fn merge(self, other: Self, info: &dyn Debug) -> Self {
        if self == other {
            self
        } else {
            eprintln!("line setting mismatch {:?} {:?} {:?}", self, other, info);
            LineSetting::Normal
        }
    }
}

impl Rune {
    pub fn new(background: Style) -> Self {
        Rune {
            text: ArrayString::from(" ").unwrap(),
            style: background,
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


impl Screen {
    pub fn new(size: (isize, isize), background: Style) -> Self {
        Screen {
            title: "".to_string(),
            rows: vec![Row::new(size.0, background); size.1 as usize + 1],
            background,
        }
    }
    pub fn default_rune(&self) -> Rune {
        Rune { text: ArrayString::from(" ").unwrap(), style: self.background }
    }
    pub fn clear(&mut self) {
        let def = self.default_rune();
        for row in self.rows.iter_mut() {
            row.runes.fill(def.clone());
        }
    }
    pub fn size(&self) -> (isize, isize) {
        (self.rows[0].runes.len() as isize - 1, self.rows.len() as isize - 1)
    }

    pub fn title(&mut self) -> &mut String {
        &mut self.title
    }
}

impl Row {
    pub fn new(width: isize, background: Style) -> Self {
        Row { runes: vec![Rune::new(background); width as usize + 1], line_setting: Default::default() }
    }
    pub fn rune_mut(&mut self, x: isize) -> &mut Rune {
        &mut self.runes[x as usize]
    }

    pub fn write(&mut self, x: isize, dx: isize, text: &str, style: Style) {
        let text = ArrayString::from(text).unwrap_or_else(|_| {
            ArrayString::from("�").unwrap()
        });
        *self.rune_mut(x) = Rune {
            text,
            style,
        };
        for x1 in x + 1..(x + dx).min(self.runes.len() as isize) {
            self.runes[x1 as usize] = Rune {
                text: ArrayString::new(),
                style,
            };
        }
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
        for (y, row) in self.rows.iter().enumerate() {
            write!(f, "{:?} {:?} ", y, row.line_setting)?;
            for x in row.runes.iter() {
                if &x.text == "" {
                    write!(f, " ")?;
                } else {
                    write!(f, "{:?}", x)?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[test]
fn test_rune() {
    assert_eq!(size_of::<Rune>(), 16);
}