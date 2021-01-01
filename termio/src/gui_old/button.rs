use std::{iter, ops, fmt};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::mem::swap;

use strum::IntoEnumIterator;
use util::rect::Rect;
use util::shared::Shared;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::gui_old::{InputEvent, OutputEvent};
use crate::gui_old::node::{Node, NodeHeader, NodeImpl};
use crate::gui_old::node::NodeExt;
use crate::input::{Event, Mouse};
use crate::output::{Background, Foreground};
use crate::screen::Style;
use std::any::Any;
use std::sync::Arc;

#[derive(Eq, Ord, PartialOrd, PartialEq, Hash, Debug, Copy, Clone)]
pub struct ButtonState {
    over: bool,
    down: bool,
}

pub struct TextButton {
    header: NodeHeader,
    state: ButtonState,
    text: String,
    styles: BTreeMap<ButtonState, Style>,
    output_event: Arc<dyn OutputEvent>,
}

impl fmt::Debug for TextButton {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TextButton")
            .field("header", &self.header)
            .field("state", &self.state)
            .field("text", &self.text)
            .field("styles", &self.styles)
            .field("callback", &"opaque")
            .finish()
    }
}

pub trait ButtonNode: NodeImpl {
    fn button_state(&self) -> &ButtonState;
    fn button_state_mut(&mut self) -> &mut ButtonState;
}


pub fn handle_button(this: &mut dyn ButtonNode, event: &InputEvent) -> bool {
    let mut new = *this.button_state();
    match event {
        InputEvent::MouseEvent(e) => {
            new.over = e.position.0 >= 0 && e.position.1 >= 0 && e.position.0 < this.size().0 && e.position.1 < this.size().1;
            new.down = e.mouse == Mouse::Down(0);
        }
        _ => {}
    }
    let old = *this.button_state();
    let clicked = old.down && !new.down && new.over;
    if old != new {
        *this.button_state_mut() = new;
        this.header_mut().mark_dirty();
    }
    return clicked;
}

impl ButtonNode for TextButton {
    fn button_state(&self) -> &ButtonState {
        &self.state
    }
    fn button_state_mut(&mut self) -> &mut ButtonState {
        &mut self.state
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
        w.style = self.styles.get(&self.state).cloned().unwrap_or(w.style);
        w.draw((0, 0), &iter::once('▛').chain(iter::repeat('▀').take(self.text.len())).chain(iter::once('▜')).collect::<String>());
        w.draw((0, 1), &format!("▌{}▐", self.text, ));
        w.draw((0, 2), &iter::once('▙').chain(iter::repeat('▄').take(self.text.len())).chain(iter::once('▟')).collect::<String>());
    }

    fn handle(&mut self, event: &InputEvent, output: &mut Vec<Arc<dyn OutputEvent>>) {
        if handle_button(self, event){
            output.push(self.output_event.clone());
        }
    }

    fn size(&self) -> (isize, isize) {
        ((self.text.len() + 2) as isize, 3)
    }
}

impl ButtonState {
    pub fn new() -> Self {
        ButtonState {
            over: false,
            down: false,
        }
    }
}

impl TextButton {
    pub fn new(text: String, output_event: Arc<dyn OutputEvent>) -> Node<TextButton> {
        let base = Style {
            foreground: Color::RGB666(0, 0, 0),
            ..Style::default()
        };
        let styles = vec![
            (ButtonState { over: false, down: false }, Style {
                background: Color::Gray24(15),
                ..base
            }),
            (ButtonState { over: false, down: true }, Style {
                background: Color::Gray24(15),
                ..base
            }),
            (ButtonState { over: true, down: false }, Style {
                background: Color::Gray24(18),
                ..base
            }),
            (ButtonState { over: true, down: true }, Style {
                background: Color::Gray24(23),
                ..base
            }),
        ].into_iter().collect();
        Self::new_internal(|header| TextButton {
            header,
            state: ButtonState::new(),
            text,
            styles,
            output_event,
        })
    }
}