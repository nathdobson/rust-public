use std::{fmt, iter, mem};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet, BTreeMap, HashMap};
use std::io::{BufWriter, Write};
use std::marker::Unsize;
use std::ops::{CoerceUnsized, Deref, Range, DerefMut};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use itertools::Itertools;
use util::bag::{Bag, Token};
use util::grid;
use util::rect::Rect;
use util::shared::{SharedMut, WkSharedMut};
use util::shared::Shared;

use crate::canvas::Canvas;
use crate::color::Color;
use crate::input::{Event, KeyEvent};
use crate::input::MouseEvent;
use crate::input::Mouse;
use crate::output::*;
use crate::gui::node::Node;
use crate::screen::{Style, Screen, LineSetting};
use crate::writer::TermWriter;
use util::io::{SafeWrite, PipelineWriter};
use std::any::Any;

pub mod button;
pub mod label;
pub mod node;
pub mod container;

pub struct Gui {
    size: (isize, isize),
    node: Node,
    pub keyboard_focus: Option<Node>,
    style: Style,
    title: String,
    output: TermWriter,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum InputEvent {
    MouseEvent(MouseEvent),
    KeyEvent(KeyEvent),
}

pub trait OutputEvent: Any + 'static + Send + Sync + fmt::Debug {
    fn as_any(&self) -> &(dyn 'static + Any);
}

impl Gui {
    pub fn new(node: Node, output: PipelineWriter) -> Gui {
        let gui = Gui {
            size: (0, 0),
            node: node,
            keyboard_focus: None,
            style: Style::default(),
            title: "".to_string(),
            output: TermWriter::new(output),
        };
        gui
    }
    pub fn close(&mut self) {
        self.output.close();
    }

    pub fn paint(&mut self) {
        let mut borrow = self.node.borrow_mut();
        if !borrow.check_dirty() {
            return;
        }
        let mut screen = Screen::new();
        screen.title = self.title.clone();
        for y in 1..borrow.size().1 {
            let line_setting = borrow.line_setting(y);
            if let Some(line_setting) = line_setting {
                screen.row(y as isize).line_setting = line_setting;
            }
        }
        let canvas = Canvas::new(&mut screen, borrow.bounds(), self.style);
        borrow.paint(canvas);
        self.output.render(&screen, &self.style);
    }

    pub fn flush(&mut self) {
        self.output.flush();
    }

    pub fn set_background(&mut self, style: Style) {
        if self.style != style {
            self.style = style;
            self.node.borrow_mut().header_mut().mark_dirty();
        }
    }

    pub fn set_title(&mut self, title: String) {
        if self.title != title {
            self.title = title;
            self.node.borrow_mut().header_mut().mark_dirty();
        }
    }

    pub fn handle(&mut self, event: &Event, output: &mut Vec<Arc<dyn OutputEvent>>) {
        match event {
            Event::KeyEvent(key_event) => {
                if let Some(focus) = self.keyboard_focus.as_mut() {
                    focus.borrow_mut().handle(&InputEvent::KeyEvent(*key_event), output);
                }
            }
            Event::MouseEvent(mouse_event) =>
                {
                    let mut mouse_event = mouse_event.clone();
                    if let Some(line_setting) = self.node.borrow().line_setting(mouse_event.position.1) {
                        if line_setting != LineSetting::Normal {
                            mouse_event.position.0 *= 2;
                        }
                    }
                    self.node.borrow_mut().handle(&InputEvent::MouseEvent(mouse_event), output);
                }
            Event::TextAreaSize(w, h) => {
                println!("Window size {} {}", w, h);
                self.size = (*w, *h);
            }
            _ => {}
        }
    }
}
//match mouse_event.mouse {
//Mouse::ScrollUp => {}
//Mouse::ScrollDown => {}
//Mouse::Up => {
//let old_focus = self.mouse_focus.take();
//let new_focus = self.node_at(mouse_event.position);
//self.mouse_focus = new_focus.clone();
//match (old_focus, new_focus) {
//(Some(old_focus), Some(new_focus)) =>
//if &old_focus == &new_focus {
//return new_focus.borrow_mut().handle(event);
//} else {
//assert!(old_focus.borrow_mut().handle(event).is_none());
//return new_focus.borrow_mut().handle(event);
//}
//(Some(old_focus), None) => {
//return old_focus.borrow_mut().handle(event);
//}
//(None, Some(new_focus)) => {
//return new_focus.borrow_mut().handle(event);
//}
//(None, None) => {}
//}
//}
//Mouse::Down(n) => {
//self.mouse_focus = self.mouse_focus.take().or_else(|| self.node_at(mouse_event.position));
//if let Some(mouse_focus) = self.mouse_focus.clone() {
//return mouse_focus.borrow_mut().handle(event);
//}
//}
//}
