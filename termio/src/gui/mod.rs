use std::{fmt, iter};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet};
use std::io::{BufWriter, Write};
use std::marker::Unsize;
use std::ops::{CoerceUnsized, Deref, Range};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use itertools::Itertools;
use util::bag::{Bag, Token};
use util::shared::{HasHeaderExt, SharedMut, WkSharedMut};

use crate::canvas::{Canvas, LineSetting, Rectangle};
use crate::color::Color;
use crate::gui::button::Button;
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
                    VideoPush};
use crate::output::{AlternateEnable, Background, CursorRestore, CursorSave, FocusTrackingEnable, Foreground, ReportWindowSize};
use crate::write;
use crate::write::SafeWrite;
use util::shared::Shared;

pub mod button;
pub mod label;

#[derive(Eq, Ord, PartialEq, PartialOrd, Debug, Hash)]
enum GuiState {
    Starting,
    Painting,
    Closing,
    Closed,
}

pub struct Gui {
    size: (isize, isize),
    nodes: HashSet<DynNode>,
    pub keyboard_focus: Option<DynNode>,
    pub mouse_focus: Option<DynNode>,
    pub background: Option<Color>,
    state: GuiState,
}

pub type Node<T> = SharedMut<T>;
pub type DynNode = Node<dyn IsNode>;

#[derive(Debug)]
pub struct NodeHeader {
    pub position: (isize, isize),
}

pub trait IsNode: HasHeaderExt<NodeHeader> + Send + Sync + 'static + fmt::Debug {
    fn paint(&self, w: &mut Canvas);
    fn handle_event(&mut self, event: &Event) -> Option<NodeEvent>;
    fn size(&self) -> (isize, isize);
    fn position(&self) -> (isize, isize) {
        self.header().position
    }
    fn bounds(&self) -> Rectangle {
        Rectangle {
            position: self.position(),
            size: self.size(),
        }
    }
    fn line_setting(&self, y: isize) -> Option<LineSetting> {
        Some(LineSetting::Normal)
    }
}


#[derive(Debug)]
pub enum NodeEvent {
    Button(Node<Button>),
}

impl NodeHeader {
    pub fn new() -> Self {
        NodeHeader {
            position: (0, 0),
        }
    }
}

impl Gui {
    pub fn new() -> Gui {
        let gui = Gui {
            size: (0, 0),
            nodes: HashSet::new(),
            keyboard_focus: None,
            mouse_focus: None,
            background: None,
            state: GuiState::Starting,
        };
        gui
    }
    pub fn close(&mut self) {
        self.state = GuiState::Closed;
    }

    pub fn add_node(&mut self, node: DynNode) {
        self.nodes.insert(node);
    }

    pub fn paint(&mut self, writer: &mut dyn SafeWrite) {
        if self.state == GuiState::Starting {
            swrite!(writer, "{}", AllMotionTrackingEnable);
            swrite!(writer, "{}", FocusTrackingEnable);
            swrite!(writer, "{}", ReportWindowSize);
            swrite!(writer, "{}", AlternateEnable);
            self.state = GuiState::Painting;
        }
        if self.state == GuiState::Closing {
            swrite!(writer, "{}", AllMotionTrackingDisable);
            swrite!(writer, "{}", FocusTrackingDisable);
            swrite!(writer, "{}", AlternateDisable);
            self.state = GuiState::Closed;
        }
        if self.state == GuiState::Closed {
            return;
        }
        swrite!(writer, "{}", CursorSave);
        if let Some(background) = self.background {
            swrite!(writer, "{}", Background(background));
        }
        swrite!(writer, "{}", EraseAll);
        let line_settings: Vec<LineSetting> = (0..self.size.1).map(|y| {
            let for_line: BTreeSet<LineSetting> = self.nodes.iter().filter_map(|node| {
                let node = node.borrow();
                if node.bounds().ys().contains(&y) {
                    node.line_setting(y - node.bounds().position.1)
                } else { None }
            }).collect();
            if for_line.len() > 1 {
                eprintln!("line {:?} has line_settings {:?}", y, for_line);
            }
            for_line.into_iter().min().unwrap_or(LineSetting::Normal)
        }).collect();
        for (y, setting) in line_settings.iter().enumerate() {
            swrite!(writer, "{}", CursorPosition(0, y as isize));
            match setting {
                LineSetting::Normal => {}
                LineSetting::DoubleHeightTop => swrite!(writer, "{}", DoubleHeightTop),
                LineSetting::DoubleHeightBottom => swrite!(writer, "{}", DoubleHeightBottom),
            }
        }
        for node in self.nodes.iter() {
            swrite!(writer, "{}", VideoNormal);
            let borrow = node.deref().borrow_mut();
            let mut canvas = Canvas {
                writer: &mut *writer,
                bounds: borrow.bounds(),
                line_settings: &line_settings,
            };
            borrow.paint(&mut canvas);
        }
        swrite!(writer, "{}", CursorRestore);
        writer.safe_flush();
    }

    pub fn node_at(&self, position: (isize, isize)) -> Option<DynNode> {
        for node in self.nodes.iter() {
            let borrow = node.borrow();
            if node.borrow().bounds().contains(position) {
                return Some(node.clone());
            }
        }
        None
    }

    pub fn handle_event(&mut self, event: &Event) -> Option<NodeEvent> {
        match event {
            Event::KeyEvent(key_event) => {
                if let Some(focus) = self.keyboard_focus.as_mut() {
                    return focus.borrow_mut().handle_event(event);
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
                                    return new_focus.borrow_mut().handle_event(event);
                                } else {
                                    assert!(old_focus.borrow_mut().handle_event(event).is_none());
                                    return new_focus.borrow_mut().handle_event(event);
                                }
                            (Some(old_focus), None) => {
                                return old_focus.borrow_mut().handle_event(event);
                            }
                            (None, Some(new_focus)) => {
                                return new_focus.borrow_mut().handle_event(event);
                            }
                            (None, None) => {}
                        }
                    }
                    Mouse::Down(n) => {
                        self.mouse_focus = self.mouse_focus.take().or_else(|| self.node_at(mouse_event.position));
                        if let Some(mouse_focus) = self.mouse_focus.clone() {
                            return mouse_focus.borrow_mut().handle_event(event);
                        }
                    }
                }
            }
            Event::WindowSize(w, h) => {
                self.size = (*w, *h);
            }
            _ => {}
        }
        None
    }
}

