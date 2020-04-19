use crate::output::CursorPosition;
use std::collections::HashMap;
use std::ops::Range;
use crate::write::SafeWrite;

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum LineSetting {
    Normal,
    DoubleHeightTop,
    DoubleHeightBottom,
}

pub struct Canvas<'a> {
    pub writer: &'a mut dyn SafeWrite,
    pub bounds: Rectangle,
    pub line_settings: &'a [LineSetting],
}

#[derive(Eq, PartialEq, PartialOrd, Hash, Copy, Clone)]
pub struct Rectangle {
    pub position: (isize, isize),
    pub size: (isize, isize),
}

impl Rectangle {
    pub fn contains(&self, (x, y): (isize, isize)) -> bool {
        (self.position.0..self.position.0 + self.size.0).contains(&x) && (self.position.1..self.position.1 + self.size.1).contains(&y)
    }
    pub fn with_bounds(&self, bounds: &Rectangle) -> Rectangle {
        Rectangle {
            position: (self.position.0 + bounds.position.0, self.position.1 + bounds.position.1),
            size: (bounds.size.0.min(self.position.0 + self.size.0 - bounds.position.0),
                   bounds.size.1.min(self.position.1 + self.size.1 - bounds.position.1)),
        }
    }
    pub fn xs(&self) -> Range<isize> {
        self.position.0..self.position.0 + self.size.0
    }
    pub fn ys(&self) -> Range<isize> {
        self.position.1..self.position.1 + self.size.1
    }
}

impl<'a> Canvas<'a> {
    pub fn draw(&mut self, p: (isize, isize), text: &str) {
        swrite!(self.writer, "{}", CursorPosition(self.bounds.position.0 + p.0, self.bounds.position.1 + p.1));
        swrite!(self.writer, "{}", text);
    }
    pub fn with_bounds<'b>(&'b mut self, bounds: &Rectangle) -> Canvas<'b> {
        Canvas {
            writer: &mut self.writer,
            bounds: self.bounds.with_bounds(&bounds),
            line_settings: self.line_settings,
        }
    }
}

