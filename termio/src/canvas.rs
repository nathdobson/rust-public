use crate::output::CursorPosition;
use std::collections::{HashMap, BTreeMap, BTreeSet};
use std::ops::Range;
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
use std::fmt::Formatter;

pub struct Canvas<'a> {
    screen: &'a mut Screen,
    bounds: Rect,
    pub style: Style,
}

impl<'a> Canvas<'a> {
    pub fn new(screen: &'a mut Screen, bounds: Rect, style: Style) -> Self {
        Canvas { screen, bounds, style }
    }
    //    pub fn clear(&mut self) {
//        for x in grid::cells_by_row(self.grid.as_mut()) {
//            x.style = self.style;
//            x.text.clear();
//            x.text.push_str(" ");
//        }
//    }
    pub fn draw(&mut self, p: (isize, isize), text: &str) {
        let row = self.screen.row(p.1 + self.bounds.ys().start);
        let mut x = p.0 + self.bounds.xs().start;
        if row.line_setting != LineSetting::Normal {
            x = (x - 1) / 2 + 1;
        }
        for grapheme in UnicodeSegmentation::graphemes(text, true) {
            let dx = advance(grapheme);
            row.runes.erase_and_insert(x..x + dx, Rune {
                text: grapheme.to_string(),
                style: self.style,
            });
            x += dx;
        }
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