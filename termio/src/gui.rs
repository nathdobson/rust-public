use std::ops::Range;
use std::sync::Arc;

use util::bag::{Bag, Token};

use crate::input::Event;
use crate::output::{CursorRestore, CursorSave, SafeWrite};

pub struct Gui {
    size: (isize, isize),
    nodes: Bag<Box<dyn Node>>,
    writer: Box<dyn SafeWrite>,
    focus: Option<NodeToken>,
}

pub struct NodeHeader {
    pub token: Option<NodeToken>,
    pub bounds: Rectangle,
}

pub trait Node {
    fn header(&self) -> &NodeHeader;
    fn header_mut(&mut self) -> &mut NodeHeader;
    fn paint(&self, w: &mut Canvas);
    fn handle_event(&mut self, event: Event) -> Option<NodeEvent>;
}

#[derive(Eq, PartialEq, PartialOrd, Hash, Copy, Clone)]
pub struct Rectangle {
    position: (isize, isize),
    size: (isize, isize),
}

pub struct Canvas<'a> {
    writer: &'a mut dyn SafeWrite,
    bounds: Rectangle,
}

pub type NodeToken = Token;

pub enum NodeEvent {
    Button(NodeToken),
    Enter(NodeToken),
}

impl NodeHeader {
    pub fn token(&self) -> NodeToken {
        self.token.unwrap()
    }
}

impl Rectangle {
    fn contains(&self, (x, y): (isize, isize)) -> bool {
        (self.position.0..self.size.0).contains(&x) && (self.position.1..self.size.1).contains(&y)
    }
}

pub struct Button {
    header: NodeHeader,
    text: String,
}

impl Node for Button {
    fn header(&self) -> &NodeHeader { &self.header }

    fn header_mut(&mut self) -> &mut NodeHeader { &mut self.header }

    fn paint(&self, w: &mut Canvas) {
        write!(w.writer, "[{}]", self.text);
    }

    fn handle_event(&mut self, event: Event) -> Option<NodeEvent> {
        match event {
            Event::MouseEvent(e) => Some(NodeEvent::Button(self.header.token())),
            _ => None,
        }
    }
}

impl Gui {
    pub fn new(writer: Box<dyn SafeWrite>) -> Gui {
        Gui {
            size: (0, 0),
            nodes: Bag::new(),
            writer,
            focus: None,
        }
    }
    pub fn paint(&mut self, writer: &mut dyn SafeWrite) {
        write!(self.writer, "{}", CursorSave);
        for node in self.nodes.iter_mut() {
            let mut canvas = Canvas { writer, bounds: node.header().bounds };
            node.paint(&mut canvas);
        }
        write!(self.writer, "{}", CursorRestore);
    }
    pub fn handle_event(&mut self, event: Event) -> Option<NodeEvent> {
        match event {
            Event::KeyEvent(key_event) => {
                if let Some(focus) = self.focus {
                    return self.nodes[focus].handle_event(event);
                }
            }
            Event::MouseEvent(mouse_event) => {
                for node in self.nodes.iter_mut() {
                    if node.header().bounds.contains(mouse_event.position) {
                        return node.handle_event(event);
                    }
                }
            }
            Event::WindowSize(w, h) => {
                self.size = (w, h);
            }
            _ => {}
        }
        None
    }
}