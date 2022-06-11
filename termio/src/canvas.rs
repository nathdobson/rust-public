use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, Range};

use arrayvec::ArrayString;
use itertools::Format;
use unicode_segmentation::UnicodeSegmentation;
use util::grid;
use util::grid::{Grid, GridSliceIndex, GridSliceMut};
use util::rect::Rect;

use crate::advance::advance_of_grapheme;
use crate::color::Color;
use crate::image::Image;
use crate::output::{
    Background, CursorPosition, DoubleHeightBottom, DoubleHeightTop, Foreground, SingleWidthLine,
};
use crate::screen::{LineSetting, Rune, Screen, Style};
use crate::string::{StyleFormat, StyleFormatter, StyleOption, StyleWrite};
use crate::writer::TermWriter;

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
    pub fn new(
        screen: &'a mut Screen,
        bounds: Rect,
        position: (isize, isize),
        style: Style,
    ) -> Self {
        Canvas {
            screen,
            bounds,
            position,
            style,
        }
    }
    pub fn set(&mut self, p: (isize, isize), mut grapheme: &str) -> isize {
        if grapheme == "\x1b" {
            grapheme = "�";
        }
        assert!(UnicodeSegmentation::graphemes(grapheme, true).count() == 1);
        let p = (p.0 + self.position.0, p.1 + self.position.1);
        if !self.bounds.contains(p) {
            return 0;
        }
        let row = &mut self.screen.rows[p.1 as usize];
        let mut x = p.0;
        if row.line_setting != LineSetting::Normal {
            x = (x - 1) / 2 + 1;
        }
        let dx = advance_of_grapheme(grapheme);
        row.write(x, dx, grapheme, self.style);
        x += dx;
        if row.line_setting != LineSetting::Normal {
            x = (x - 1) * 2 + 1;
        }
        x - self.position.0
    }
    pub fn draw(&mut self, p: (isize, isize), item: &dyn StyleFormat) {
        item.style_format(StyleFormatter::new(&mut self.push_translate(p)));
    }
    pub fn draw_image(&mut self, p: (isize, isize), image: &Image) {
        for (y, row) in image.rows().iter().enumerate() {
            self.draw((p.0, p.1 + y as isize), &row.string);
        }
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
