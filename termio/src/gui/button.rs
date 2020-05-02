use crate::color::Color;
use std::{iter, ops};
use crate::input::{Event, Mouse};
use crate::canvas::Canvas;
use crate::output::{Background, Foreground};
use std::mem::swap;
use util::shared::Shared;
use crate::gui::GuiEvent;
use crate::gui::node::{Node, NodeHeader, NodeImpl};
use crate::gui::node::NodeExt;
use std::cell::RefCell;

#[derive(Eq, Ord, PartialOrd, PartialEq, Hash, Debug, Copy, Clone)]
pub enum ButtonState {
    Default,
    Over,
    Down,
}

#[derive(Debug)]
pub struct TextButton {
    header: NodeHeader,
    state: ButtonState,
    text: String,
}

impl ButtonState {
    fn handle(&mut self, node: &mut dyn NodeImpl, event: &Event) -> Option<GuiEvent> {
        let mut new_state = *self;
        let mut result: Option<GuiEvent> = None;
        match event {
            Event::MouseEvent(e) => {
                if e.motion {
                    if node.bounds().contains(e.position) {
                        if e.mouse == Mouse::Down(0) {
                            new_state = ButtonState::Down;
                        } else if e.mouse == Mouse::Up {
                            new_state = ButtonState::Over;
                        }
                    } else {
                        if e.mouse == Mouse::Up {
                            new_state = ButtonState::Default;
                        } else if e.mouse == Mouse::Down(0) {
                            new_state = ButtonState::Default;
                        }
                    }
                } else {
                    if node.bounds().contains(e.position) {
                        if e.mouse == Mouse::Down(0) {
                            new_state = ButtonState::Down;
                        }
                        if e.mouse == Mouse::Up && *self != ButtonState::Default {
                            new_state = ButtonState::Over;
                            result = Some(GuiEvent::Button(node.header().this()));
                        }
                    } else {
                        if e.mouse == Mouse::Up {
                            new_state = ButtonState::Default;
                        } else if e.mouse == Mouse::Down(0) {
                            new_state = ButtonState::Default;
                        }
                    }
                }
            }
            _ => {}
        }
        if new_state != *self {
            *self = new_state;
            node.header_mut().mark_dirty();
        }
        return result;
    }
}

impl NodeImpl for TextButton {
    fn header(&self) -> &NodeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut NodeHeader {
        &mut self.header
    }

    fn paint(&self, mut w: Canvas) {
        let (fg, bg) = match self.state {
            ButtonState::Default => (0, 15),
            ButtonState::Over => (0, 18),
            ButtonState::Down => (0, 23)
        };
        w.style.background = Color::Gray24(bg);
        w.style.foreground = Color::Gray24(fg);
        w.draw((0, 0), &iter::once('▛').chain(iter::repeat('▀').take(self.text.len())).chain(iter::once('▜')).collect::<String>());
        w.draw((0, 1), &format!("▌{}▐", self.text, ));
        w.draw((0, 2), &iter::once('▙').chain(iter::repeat('▄').take(self.text.len())).chain(iter::once('▟')).collect::<String>());
    }

    fn handle(&mut self, event: &Event) -> Option<GuiEvent> {
        let mut state = self.state;
        let result = state.handle(self, event);
        self.state = state;
        result
    }

    fn size(&self) -> (isize, isize) {
        ((self.text.len() + 2) as isize, 3)
    }
}

impl TextButton {
    pub fn new(text: String) -> Node<TextButton> {
        Self::new_internal(|header| TextButton {
            header,
            state: ButtonState::Default,
            text,
        })
    }
}