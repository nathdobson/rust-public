use crate::color::Color;
use std::iter;
use crate::input::{Event, Mouse};
use crate::canvas::Canvas;
use crate::output::{Background, Foreground};
use std::mem::swap;
use crate::gui::{NodeEvent, NodeHeader, IsNode, Node};
use util::shared::{HasHeader, Header, Shared};

#[derive(Eq, Ord, PartialOrd, PartialEq, Hash, Debug)]
enum State {
    Default,
    Over,
    Down,
}

#[derive(Debug)]
pub struct Button {
    header: Header<Button, NodeHeader>,
    text: String,
    state: State,
}

impl HasHeader<NodeHeader> for Button {
    fn shared_header(&self) -> &Header<Self, NodeHeader> { &self.header }
    fn shared_header_mut(&mut self) -> &mut Header<Self, NodeHeader> { &mut self.header }
}

impl IsNode for Button {
    fn paint(&self, w: &mut Canvas) {
        let (fg, bg) = match self.state {
            State::Default => (0, 15),
            State::Over => (0, 18),
            State::Down => (0, 23)
        };
        swrite!(w.writer, "{}", Background(Color::Gray24(bg)));
        swrite!(w.writer, "{}", Foreground(Color::Gray24(fg)));
        w.draw((0, 0), &iter::once('▛').chain(iter::repeat('▀').take(self.text.len())).chain(iter::once('▜')).collect::<String>());
        w.draw((0, 1), &format!("▌{}▐", self.text, ));
        w.draw((0, 2), &iter::once('▙').chain(iter::repeat('▄').take(self.text.len())).chain(iter::once('▟')).collect::<String>());
    }

    fn handle_event(&mut self, event: &Event) -> Option<NodeEvent> {
        match event {
            Event::MouseEvent(e) => {
                if e.motion {
                    if self.bounds().contains(e.position) {
                        if e.mouse == Mouse::Down(0) {
                            self.state = State::Down;
                        } else if e.mouse == Mouse::Up {
                            self.state = State::Over;
                        }
                    } else {
                        if e.mouse == Mouse::Up {
                            self.state = State::Default;
                        } else if e.mouse == Mouse::Down(0) {
                            self.state = State::Default;
                        }
                    }
                } else {
                    if self.bounds().contains(e.position) {
                        if e.mouse == Mouse::Down(0) {
                            self.state = State::Down;
                        }
                        if e.mouse == Mouse::Up && self.state != State::Default {
                            self.state = State::Over;
                            return Some(NodeEvent::Button(self.this()));
                        }
                    } else {
                        if e.mouse == Mouse::Up {
                            self.state = State::Default;
                        } else if e.mouse == Mouse::Down(0) {
                            self.state = State::Default;
                        }
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
    pub fn new(text: String) -> Node<Button> {
        Header::new_shared(Button {
            header: Header::new_header(NodeHeader::new()),
            text,
            state: State::Default,
        })
    }
}