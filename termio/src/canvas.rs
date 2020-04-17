use crate::output::{SafeWrite, CursorPosition};

pub struct Canvas<'a> {
    pub writer: &'a mut dyn SafeWrite,
    pub bounds: Rectangle,
}

#[derive(Eq, PartialEq, PartialOrd, Hash, Copy, Clone)]
pub struct Rectangle {
    pub position: (isize, isize),
    pub size: (isize, isize),
}

impl<'a> Canvas<'a> {
    pub fn draw(&mut self, p: (isize, isize), text: &str) {
        write!(self.writer, "{}", CursorPosition(self.bounds.position.0 + p.0, self.bounds.position.1 + p.1));
        write!(self.writer, "{}", text);
    }
}

