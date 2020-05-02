use crate::output::CursorPosition;
use std::collections::{HashMap, BTreeMap, BTreeSet};
use std::ops::Range;
use crate::write::SafeWrite;
use crate::color::Color;
use util::grid::{Grid, GridSliceMut, GridSliceIndex};
use util::rect::Rect;
use unicode_segmentation::UnicodeSegmentation;
use util::grid;
use crate::output::Background;
use crate::output::Foreground;
use crate::output::DoubleHeightTop;
use crate::output::DoubleHeightBottom;
use crate::output::SingleWidthLine;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Screen {
    grid: Grid<Cell>,
}

pub struct Canvas<'a> {
    grid: GridSliceMut<'a, Cell>,
    pub style: Style,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub struct Cell {
    text: String,
    style: Style,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Default)]
pub struct Style {
    pub background: Color,
    pub foreground: Color,
    pub line_setting: LineSetting,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub enum LineSetting {
    Normal,
    DoubleHeightTop,
    DoubleHeightBottom,
}

impl Default for LineSetting {
    fn default() -> Self {
        LineSetting::Normal
    }
}

impl Cell {
    fn new() -> Self {
        Cell {
            text: " ".to_string(),
            style: Default::default(),
        }
    }
}

pub fn advance(string: &str, line_setting: LineSetting) -> isize {
    let line_factor = if line_setting == LineSetting::Normal { 1 } else { 2 };
    line_factor * match string {
        "" => 0,
        "Ａ" | "Ｋ" | "Ｑ" | "Ｊ" | "🐕" | "🦅" | "🐉" => 2,
        _ => 1,
    }
}

struct FrameBuilder<'a> {
    inner: &'a mut Vec<u8>,
    cursor: (isize, isize),
    style: Style,
}

impl<'a> FrameBuilder<'a> {
    fn new(inner: &'a mut Vec<u8>) -> Self {
        swrite!(inner, "{}", CursorPosition(1,1));
        swrite!(inner, "{}", Background(Color::Default));
        swrite!(inner, "{}", SingleWidthLine);
        FrameBuilder {
            inner,
            cursor: (1, 1),
            style: Style {
                background: Default::default(),
                foreground: Default::default(),
                line_setting: Default::default(),
            },
        }
    }
    pub fn move_cursor(&mut self, x: isize, y: isize) {
        self.cursor = (x, y);
        swrite!(self.inner, "{}", CursorPosition(x, y));
    }
    pub fn set_line_setting(&mut self, setting: LineSetting) {
        self.style.line_setting = setting;
        match setting {
            LineSetting::Normal => swrite!(self.inner, "{}", SingleWidthLine),
            LineSetting::DoubleHeightTop => swrite!(self.inner, "{}", DoubleHeightTop),
            LineSetting::DoubleHeightBottom => swrite!(self.inner, "{}", DoubleHeightBottom),
        }
    }
    pub fn set_style(&mut self, style: &Style) {
        if style.foreground != self.style.foreground {
            swrite!(self.inner,"{}",Foreground(style.foreground));
            self.style.foreground = style.foreground;
        }
        if style.background != self.style.background {
            swrite!(self.inner,"{}",Background(style.background));
            self.style.background = style.background;
        }
    }
    pub fn write(&mut self, text: &str) {
        swrite!(self.inner, "{}", text);
        self.cursor.0 += advance(text, self.style.line_setting);
    }
}

impl Screen {
    pub fn new(size: (isize, isize)) -> Self {
        Screen {
            grid: Grid::new(size, |_, _| Cell::new())
        }
    }
    pub fn size(&self) -> (isize, isize) {
        self.grid.size()
    }
    pub fn flush(&self, w: &mut Vec<u8>) {
        let mut builder = FrameBuilder::new(w);
        for (y, row) in grid::rows(self.grid.as_ref()).enumerate() {
            if (y as isize) == 0 {
                continue;
            }
            builder.move_cursor(1, y as isize);
            let settings: BTreeSet<_> = grid::cells_by_col(row).filter_map(|cell| {
                if &cell.text == " " {
                    None
                } else {
                    Some(cell.style.line_setting)
                }
            }).collect();
            if settings.len() > 1 {
                eprintln!("Could not render {} because of {:?}", y, settings);
            }
            let setting = settings.iter().last().cloned().unwrap_or(LineSetting::Normal);
            builder.set_line_setting(setting);
            for (x, cell) in grid::cells_by_col(row).enumerate() {
                let x = x as isize;
                if x < builder.cursor.0 {
                    if &cell.text != " " {
                        eprintln!("Hidden text {:?} at {} {}", cell.text, x, builder.cursor.0);
                    }
                    continue;
                }
                assert_eq!(x, builder.cursor.0);
                if !cell.text.is_empty() {
                    builder.set_style(&cell.style);
                    builder.write(&cell.text);
                }
            }
        }
    }
}

impl<'a> Canvas<'a> {
    pub fn new(screen: &'a mut Screen, style: Style) -> Self {
        Canvas { grid: screen.grid.as_mut(), style }
    }
    pub fn clear(&mut self) {
        for x in grid::cells_by_row(self.grid.as_mut()) {
            x.style = self.style;
            x.text.clear();
            x.text.push_str(" ");
        }
    }
    pub fn draw(&mut self, p: (isize, isize), text: &str) {
        let mut x = p.0;
        for grapheme in UnicodeSegmentation::graphemes(text, true) {
            let dx = advance(grapheme, self.style.line_setting);
            if let Some(cell) = self.grid.get_mut((x, p.1)) {
                cell.text.clear();
                cell.text.push_str(grapheme);
                cell.style = self.style.clone();
            }
            x += dx;
        }
    }
    pub fn push_bounds<'b>(&'b mut self, bounds: Rect) -> Canvas<'b> {
        Canvas {
            grid: self.grid.as_mut().with_bounds_truncated(bounds),
            style: self.style.clone(),
        }
    }
    pub fn push<'b>(&'b mut self) -> Canvas<'b> {
        Canvas {
            grid: self.grid.as_mut(),
            style: self.style.clone(),
        }
    }
}

