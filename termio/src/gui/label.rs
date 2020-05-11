use util::shared::{Shared, SharedMut};
use crate::canvas::Canvas;
use crate::input::{Event, Mouse};
use itertools::Itertools;
use crate::color::Color;
use crate::output::{Background, Foreground};
use std::ops;
use crate::gui::node::{NodeHeader, NodeImpl, Node};
use crate::gui::node::NodeExt;
use crate::screen::{LineSetting, Style};
use crate::gui::{InputEvent, OutputEvent};
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
}