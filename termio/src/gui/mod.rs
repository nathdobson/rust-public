use std::{fmt, iter, mem};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet};
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

use crate::canvas::{Canvas, LineSetting, Screen, Style};
use crate::color::Color;
use crate::input::Event;
use crate::input::Event::MouseEvent;
use crate::input::Mouse;
use crate::output::{AllMotionTrackingDisable,
                    AllMotionTrackingEnable,
                    AlternateDisable,
                    CursorPosition,
                    DoubleHeightBottom,
                    DoubleHeightTop,
                    EraseAll,
                    FocusTrackingDisable,
                    VideoNormal,
                    VideoPop,
                    VideoPush,
                    AlternateEnable,
                    Background,
                    CursorHide,
                    CursorRestore,
                    CursorSave,
                    CursorShow,
                    FocusTrackingEnable,
                    Foreground,
                    ReportTextAreaSize};
use crate::write;
use crate::write::SafeWrite;
use crate::gui::node::Node;

pub mod button;
pub mod label;
pub mod node;

#[derive(Eq, Ord, PartialEq, PartialOrd, Debug, Hash)]
enum GuiState {
    Starting,
    Painting,
    Closing,
    Closed,
}

pub struct Gui {
    size: (isize, isize),
    nodes: HashSet<Node>,
    pub keyboard_focus: Option<Node>,
    pub mouse_focus: Option<Node>,
    pub style: Style,
    state: GuiState,
    old_screen: Screen,
    new_screen: Screen,
}


#[derive(Debug)]
pub enum GuiEvent {
    Button(Node),
}

impl Gui {
    pub fn new() -> Gui {
        let gui = Gui {
            size: (0, 0),
            nodes: HashSet::new(),
            keyboard_focus: None,
            mouse_focus: None,
            style: Style::default(),
            state: GuiState::Starting,
            old_screen: Screen::new((0, 0)),
            new_screen: Screen::new((0, 0)),
        };
        gui
    }
    pub fn close(&mut self) {
        self.state = GuiState::Closed;
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node);
    }

    pub fn paint(&mut self, writer: &mut Vec<u8>) {
        if self.state == GuiState::Starting {
            swrite!(writer, "{}", AllMotionTrackingEnable);
            swrite!(writer, "{}", FocusTrackingEnable);
            swrite!(writer, "{}", ReportTextAreaSize);
            swrite!(writer, "{}", AlternateEnable);
            swrite!(writer, "{}", CursorHide);
            self.state = GuiState::Painting;
        }
        if self.state == GuiState::Closing {
            swrite!(writer, "{}", AllMotionTrackingDisable);
            swrite!(writer, "{}", FocusTrackingDisable);
            swrite!(writer, "{}", AlternateDisable);
            swrite!(writer,"{}",CursorShow);
            self.state = GuiState::Closed;
        }
        if self.state == GuiState::Closed {
            return;
        }
        let mut dirty = false;
        for node in self.nodes.iter() {
            let mut node = node.borrow_mut();
            dirty |= node.header_mut().check_dirty();
        }
        if !dirty {
            return;
        }
        if self.new_screen.size() != self.size {
            self.new_screen = Screen::new((self.size.0 + 1, self.size.1 + 1));
        }
        let mut canvas = Canvas::new(&mut self.new_screen, self.style);
        canvas.clear();
        for node in self.nodes.iter() {
            let borrow = node.deref().borrow_mut();
            let canvas = canvas.push_bounds(borrow.bounds());
            borrow.paint(canvas)
        }
        if self.new_screen != self.old_screen {
            swrite!(writer, "{}", CursorSave);
            self.new_screen.flush(writer);
            swrite!(writer, "{}", CursorRestore);
            writer.safe_flush();
            mem::swap(&mut self.old_screen, &mut self.new_screen);
        }
    }

    pub fn node_at(&self, position: (isize, isize)) -> Option<Node> {
        for node in self.nodes.iter() {
            let borrow = node.borrow();
            if node.borrow().bounds().contains(position) {
                return Some(node.clone());
            }
        }
        None
    }

    pub fn handle(&mut self, event: &Event) -> Option<GuiEvent> {
        match event {
            Event::KeyEvent(key_event) => {
                if let Some(focus) = self.keyboard_focus.as_mut() {
                    return focus.borrow_mut().handle(event);
                }
            }
            Event::MouseEvent(mouse_event) => {
                match mouse_event.mouse {
                    Mouse::ScrollUp => {}
                    Mouse::ScrollDown => {}
                    Mouse::Up => {
                        let old_focus = self.mouse_focus.take();
                        let new_focus = self.node_at(mouse_event.position);
                        self.mouse_focus = new_focus.clone();
                        match (old_focus, new_focus) {
                            (Some(old_focus), Some(new_focus)) =>
                                if &old_focus == &new_focus {
                                    return new_focus.borrow_mut().handle(event);
                                } else {
                                    assert!(old_focus.borrow_mut().handle(event).is_none());
                                    return new_focus.borrow_mut().handle(event);
                                }
                            (Some(old_focus), None) => {
                                return old_focus.borrow_mut().handle(event);
                            }
                            (None, Some(new_focus)) => {
                                return new_focus.borrow_mut().handle(event);
                            }
                            (None, None) => {}
                        }
                    }
                    Mouse::Down(n) => {
                        self.mouse_focus = self.mouse_focus.take().or_else(|| self.node_at(mouse_event.position));
                        if let Some(mouse_focus) = self.mouse_focus.clone() {
                            return mouse_focus.borrow_mut().handle(event);
                        }
                    }
                }
            }
            Event::TextAreaSize(w, h) => {
                println!("Window size {} {}", w, h);
                self.size = (*w, *h);
            }
            _ => {}
        }
        None
    }
}

