use crate::output::CursorPosition;
use std::collections::{HashMap, BTreeMap, BTreeSet};
use std::ops::{Range, Deref};
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
use crate::screen::{LineSetting, Screen, Rune, Style, advance};
use crate::writer::TermWriter;
use std::fmt;
use std::fmt::{Formatter, Display};
use crate::string::{StyleFormat, StyleWrite, StyleOption, StyleFormatter};
use itertools::Format;

pub struct Canvas<'a> {
    screen: &'a mut Screen,
    bounds: Rect,
    pub style: Style,
}

struct Cursor<'a> {
    canvas: Canvas<'a>,
    position: (isize, isize),
}

impl<'a> StyleWrite for Cursor<'a> {
    fn style_write(&mut self, style: StyleOption, value: &dyn Display) {
        let mut canvas = self.canvas.push();
        canvas.style = style.unwrap_or(canvas.style);
        for grapheme in UnicodeSegmentation::graphemes(value.to_string().as_str(), true) {
            self.position.0 = canvas.set(self.position, grapheme);
        }
    }
}

impl<'a> Canvas<'a> {
    pub fn new(screen: &'a mut Screen, bounds: Rect, style: Style) -> Self {
        Canvas { screen, bounds, style }
    }
    pub fn set(&mut self, p: (isize, isize), mut grapheme: &str) -> isize {
        if grapheme == "\x1b" {
            grapheme = "�";
        }
        assert!(UnicodeSegmentation::graphemes(grapheme, true).count() == 1);
        let row = self.screen.row(p.1 + self.bounds.ys().start);
        let mut x = p.0 + self.bounds.xs().start;
        if row.line_setting != LineSetting::Normal {
            x = (x - 1) / 2 + 1;
        }
        let dx = advance(grapheme);
        row.runes.erase_and_insert(x..x + dx, Rune {
            text: grapheme.to_string(),
            style: self.style,
        });
        x += dx;
        if row.line_setting != LineSetting::Normal {
            x = (x - 1) * 2 + 1;
        }
        x - self.bounds.xs().start
    }
    pub fn draw(&mut self, p: (isize, isize), item: &dyn StyleFormat) {
        item.style_format(StyleFormatter::new(&mut Cursor { canvas: self.push(), position: p }));
    }
    pub fn push<'b>(&'b mut self) -> Canvas<'b> {
        Canvas {
            screen: self.screen,
            style: self.style.clone(),
            bounds: self.bounds,
        }
    }
    pub fn push_bounds<'b>(&'b mut self, rect: Rect) -> Canvas<'b> {
        Canvas {
            screen: self.screen,
            style: self.style.clone(),
            bounds: self.bounds.sub_rectangle_truncated(&rect),
        }
    }
}

impl<'a> fmt::Debug for Canvas<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Canvas")
            .field("bounds", &self.bounds)
            .field("style", &self.style)
            .finish()
    }
}