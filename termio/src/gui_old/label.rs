use util::shared::{Shared, SharedMut};
use crate::canvas::Canvas;
use crate::input::{Event, Mouse};
use itertools::Itertools;
use crate::color::Color;
use crate::output::{Background, Foreground};
use std::{ops, mem};
use crate::gui_old::node::{NodeHeader, NodeImpl, Node};
use crate::gui_old::node::NodeExt;
use crate::screen::{LineSetting, Style};
use crate::gui_old::{InputEvent, OutputEvent};
use std::sync::Arc;
use util::io::SafeWrite;
use unicode_segmentation::UnicodeSegmentation;
use crate::string::StyleString;

#[derive(Debug)]
pub struct Label {
    header: NodeHeader,
    lines: Vec<StyleString>,
    scroll: isize,
    pub size: (isize, isize),
}


impl NodeImpl for Label {
    fn header(&self) -> &NodeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut NodeHeader {
        &mut self.header
    }

    fn paint(&self, mut w: Canvas) {
        for (i, y) in (self.scroll..self.scroll + self.size.1).enumerate() {
            if y >= 0 {
                if let Some(line) = self.lines.get(y as usize) {
                    w.draw((0, i as isize), &line);
                }
            }
        }
        if self.size.1 < self.lines.len() as isize {
            let top = self.scroll;
            let middle = self.size.1;
            let bottom = self.lines.len() as isize - middle - top;
            let top = ((self.size.1 - 1) as f32 * 8.0 * top as f32 / (self.lines.len() as f32)).ceil() as isize;
            let bottom = ((self.size.1 - 1) as f32 * 8.0 * bottom as f32 / (self.lines.len() as f32)).ceil() as isize;
            let middle = self.size.1 * 8 - top - bottom;
            println!("{:?} {:?} {:?}", top, middle, bottom);
            for y in 0..self.size.1 {
                if y < top / 8 {
                    w.draw((self.size.0 - 1, y), &' ');
                } else if y == top / 8 {
                    w.draw((self.size.0 - 1, y),
                           &std::char::from_u32('█' as u32 - (top % 8) as u32).unwrap());
                } else if y < (top + middle) / 8 {
                    w.draw((self.size.0 - 1, y), &'█');
                } else if y == (top + middle) / 8 {
                    let mut w2 = w.push();
                    mem::swap(&mut w2.style.background, &mut w2.style.foreground);
                    w2.draw((self.size.0 - 1, y),
                            &std::char::from_u32('█' as u32 - ((top + middle) % 8) as u32).unwrap());
                } else {
                    w.draw((self.size.0 - 1, y), &' ');
                }
            }
        }
    }

    fn handle(&mut self, event: &InputEvent, output: &mut Vec<Arc<dyn OutputEvent>>) {
        match event {
            InputEvent::MouseEvent(event) => {
                match event.mouse {
                    Mouse::ScrollDown => {
                        if self.scroll < (self.lines.len() as isize) - self.size.1 {
                            self.scroll += 1;
                            self.header_mut().mark_dirty();
                        }
                    }
                    Mouse::ScrollUp => {
                        if self.scroll > 0 {
                            self.scroll -= 1;
                            self.header_mut().mark_dirty();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn size(&self) -> (isize, isize) {
        self.size
    }
}


impl Label {
    pub fn new(lines: Vec<StyleString>, size: (isize, isize)) -> Node<Label> {
        Self::new_internal(|header| Label {
            header,
            lines,
            scroll: 0,
            size,
        })
    }
    pub fn push(&mut self, line: StyleString) {
        self.lines.push(line);
        self.scroll = (self.lines.len() as isize) - self.size.1;
        self.header_mut().mark_dirty();
    }
    pub fn clear(&mut self) {
        self.scroll = 0;
        self.lines.clear();
        self.header_mut().mark_dirty();
    }
}