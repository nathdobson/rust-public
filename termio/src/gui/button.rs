use crate::color::Color;
use std::iter;
use crate::input::{Event, Mouse};
use crate::gui::{NodeEvent, NodeHeader, Node};
use crate::canvas::Canvas;
use crate::output::{Background, Foreground};
use std::mem::swap;

pub struct Button {
    header: NodeHeader,
    text: String,
    down: bool,
}

impl Node for Button {
    fn header(&self) -> &NodeHeader { &self.header }

    fn header_mut(&mut self) -> &mut NodeHeader { &mut self.header }

    fn paint(&self, w: &mut Canvas) {
        let mut bg = 8;
        let mut fg = 23;
        if self.down {
            swap(&mut fg, &mut bg);
        }
        write!(w.writer, "{}", Background(Color::Gray24(bg)));
        write!(w.writer, "{}", Foreground(Color::Gray24(fg)));
        w.draw((0, 0), &iter::once('▛').chain(iter::repeat('▀').take(self.text.len())).chain(iter::once('▜')).collect::<String>());
        w.draw((0, 1), &format!("▌{}▐", self.text, ));
        w.draw((0, 2), &iter::once('▙').chain(iter::repeat('▄').take(self.text.len())).chain(iter::once('▟')).collect::<String>());
    }

    fn handle_event(&mut self, event: &Event) -> Option<NodeEvent> {
        match event {
            Event::MouseEvent(e) => {
                if !e.motion {
                    if self.bounds().contains(e.position) {
                        if e.mouse == Mouse::Down(0) {
                            self.down = true;
                        }
                        if e.mouse == Mouse::Up && self.down {
                            self.down = false;
                            return Some(NodeEvent::Button(self.header.token()));
                        }
                    } else if e.mouse == Mouse::Up {
                        self.down = false;
                    }
                }
            }
            _ => {}
        }
        None
    }
    fn size(&self) -> (isize, isize) {
        ((self.text.len() + 2) as isize, 3)
    }
}

impl Button {
    pub fn new(text: String) -> Button {
        Button {
            header: NodeHeader::new(),
            text,
            down: false,
        }
    }
}