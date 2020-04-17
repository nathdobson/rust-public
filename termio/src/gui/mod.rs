use std::ops::Range;
use std::sync::Arc;
use std::iter;
use crate::output::{AllMotionTrackingEnable, CursorPosition, EraseAll};

use util::bag::{Bag, Token};

use crate::input::Event;
use crate::input::Mouse;
use crate::output::{CursorRestore, CursorSave, Foreground, Background, SafeWrite, AlternateEnable, FocusTrackingEnable, ReportWindowSize};
use crate::input::Event::MouseEvent;
use crate::color::Color;
use crate::canvas::{Canvas, Rectangle};

pub mod button;

pub struct Gui {
    size: (isize, isize),
    nodes: Bag<Box<dyn Node>>,
    pub writer: Box<dyn SafeWrite + 'static + Send>,
    pub keyboard_focus: Option<NodeToken>,
    pub mouse_focus: Option<NodeToken>,
    pub background: Option<Color>,
}

pub struct NodeHeader {
    pub token: Option<NodeToken>,
    pub position: (isize, isize),
}

pub trait Node: 'static + Send {
    fn header(&self) -> &NodeHeader;
    fn header_mut(&mut self) -> &mut NodeHeader;
    fn paint(&self, w: &mut Canvas);
    fn handle_event(&mut self, event: &Event) -> Option<NodeEvent>;
    fn size(&self) -> (isize, isize);
    fn bounds(&self) -> Rectangle {
        Rectangle { position: self.header().position, size: self.size() }
    }
}

pub type NodeToken = Token;

#[derive(Debug)]
pub enum NodeEvent {
    Button(NodeToken),
    Enter(NodeToken),
}

impl NodeHeader {
    pub fn new() -> Self {
        NodeHeader {
            token: None,
            position: (0, 0),
        }
    }
    pub fn token(&self) -> NodeToken {
        self.token.unwrap()
    }
}

impl Rectangle {
    fn contains(&self, (x, y): (isize, isize)) -> bool {
        (self.position.0..self.position.0 + self.size.0).contains(&x) && (self.position.1..self.position.1 + self.size.1).contains(&y)
    }
}

impl Gui {
    pub fn new(mut writer: Box<dyn SafeWrite + 'static + Send>) -> Gui {
        write!(writer, "{}", AllMotionTrackingEnable);
        write!(writer, "{}", FocusTrackingEnable);
        write!(writer, "{}", ReportWindowSize);
        write!(writer, "{}", AlternateEnable);
        let gui = Gui {
            size: (0, 0),
            nodes: Bag::new(),
            writer,
            keyboard_focus: None,
            mouse_focus: None,
            background: None,
        };
        gui
    }
    pub fn add_node(&mut self, node: Box<dyn Node>) -> NodeToken {
        let token = self.nodes.push(node);
        self.nodes[token].header_mut().token = Some(token);
        token
    }
    pub fn paint(&mut self) {
        write!(self.writer, "{}", CursorSave);
        if let Some(background) = self.background {
            write!(self.writer, "{}", Background(background));
        }
        write!(self.writer, "{}", EraseAll);
        for (token, node) in self.nodes.iter_mut() {
            let mut canvas = Canvas { writer: &mut *self.writer, bounds: node.bounds() };
            node.paint(&mut canvas);
        }
        write!(self.writer, "{}", CursorRestore);
        self.writer.flush();
    }
    pub fn node_at(&self, position: (isize, isize)) -> Option<Token> {
        for (token, node) in self.nodes.iter() {
            if node.bounds().contains(position) {
                return Some(token);
            }
        }
        None
    }
    pub fn handle_event(&mut self, event: &Event) -> Option<NodeEvent> {
        match event {
            Event::KeyEvent(key_event) => {
                if let Some(focus) = self.keyboard_focus {
                    return self.nodes[focus].handle_event(event);
                }
            }
            Event::MouseEvent(mouse_event) => {
                match mouse_event.mouse {
                    Mouse::ScrollUp => {}
                    Mouse::ScrollDown => {}
                    Mouse::Up => {
                        if let Some(mouse_focus) = self.mouse_focus.take().or_else(|| self.node_at(mouse_event.position)) {
                            return self.nodes[mouse_focus].handle_event(event);
                        }
                    }
                    Mouse::Down(n) => {
                        self.mouse_focus = self.mouse_focus.or_else(|| self.node_at(mouse_event.position));
                        if let Some(mouse_focus) = self.mouse_focus {
                            return self.nodes[mouse_focus].handle_event(event);
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

