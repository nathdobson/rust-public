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
    position: (isize, isize),
    pub style: Style,
}

impl<'a> StyleWrite for Canvas<'a> {
    fn style_write(&mut self, style: StyleOption, value: &dyn Display) {
        let mut canvas = self.push();
        canvas.style = style.unwrap_or(canvas.style);
        for grapheme in UnicodeSegmentation::graphemes(value.to_string().as_str(), true) {
            canvas.position.0 += canvas.set((0, 0), grapheme);
        }
        self.position = canvas.position;
    }
}

impl<'a> Canvas<'a> {
    pub fn new(screen: &'a mut Screen, bounds: Rect, position: (isize, isize), style: Style) -> Self {
        Canvas { screen, bounds, position: (0, 0), style }
    }
    pub fn set(&mut self, p: (isize, isize), mut grapheme: &str) -> isize {
        if grapheme == "\x1b" {
            grapheme = "�";
        }
        assert!(UnicodeSegmentation::graphemes(grapheme, true).count() == 1);
        let p = (p.0 + self.position.0, p.1 + self.position.1);
        if !self.bounds.contains(p){
            return 0;
        }
        let row = self.screen.row(p.1);
        let mut x = p.0;
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
        x - self.position.0
    }
    pub fn draw(&mut self, p: (isize, isize), item: &dyn StyleFormat) {
        item.style_format(StyleFormatter::new(&mut self.push_translate(p)));
    }
    pub fn push<'b>(&'b mut self) -> Canvas<'b> {
        Canvas {
            screen: self.screen,
            style: self.style.clone(),
            bounds: self.bounds,
            position: self.position,
        }
    }

    pub fn push_bounds<'b>(&'b mut self, rect: Rect) -> Canvas<'b> {
        Canvas {
            screen: self.screen,
            style: self.style.clone(),
            bounds: self.bounds.sub_rectangle_truncated(&rect),
            position: self.position,
        }
    }

    pub fn push_translate<'b>(&'b mut self, position: (isize, isize)) -> Canvas<'b> {
        Canvas {
            screen: self.screen,
            style: self.style.clone(),
            bounds: self.bounds,
            position: (self.position.0 + position.0, self.position.1 + position.1),
        }
    }
}

impl<'a> fmt::Debug for Canvas<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Canvas")
            .field("bounds", &self.bounds)
            .field("position", &self.position)
            .field("style", &self.style)
            .finish()
    }
}