use std::ops::Range;
use std::sync::Arc;
use std::iter;
use crate::output::{AllMotionTrackingEnable, CursorPosition, EraseAll};

use util::bag::{Bag, Token};

use crate::input::Event;
use crate::input::Mouse;
use crate::output::{CursorRestore, CursorSave, Foreground, Background, SafeWrite, AlternateEnable, FocusTrackingEnable, ReportWindowSize};
use crate::input::Event::MouseEvent;

pub struct Gui {
    size: (isize, isize),
    nodes: Bag<Box<dyn Node>>,
    pub writer: Box<dyn SafeWrite + 'static + Send>,
    pub keyboard_focus: Option<NodeToken>,
    pub mouse_focus: Option<NodeToken>,
    pub background: Option<u8>,
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

#[derive(Eq, PartialEq, PartialOrd, Hash, Copy, Clone)]
pub struct Rectangle {
    pub position: (isize, isize),
    pub size: (isize, isize),
}

pub struct Canvas<'a> {
    writer: &'a mut dyn SafeWrite,
    bounds: Rectangle,
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

impl<'a> Canvas<'a> {
    pub fn draw(&mut self, p: (isize, isize), text: &str) {
        write!(self.writer, "{}", CursorPosition(self.bounds.position.0 + p.0, self.bounds.position.1 + p.1));
        write!(self.writer, "{}", text);
    }
}

pub struct Button {
    header: NodeHeader,
    text: String,
    down: bool,
}

//pub fn draw_box(c11: bool, c21: bool, c12: bool, c22: bool) -> char {
//    match (c11, c21, c12, c22) {
//        (false, false, false, false) => ' ',
//
//        (true, false, false, false) => '▘',
//        (false, true, false, false) => '▝',
//        (false, false, true, false) => '▖',
//        (false, false, false, true) => '▗',
//
//        (true, true, false, false) => '▀',
//        (false, false, true, true) => '▄',
//        (true, false, true, false) => '▌',
//        (false, true, false, true) => '▐',
//
//        (true, false, false, true) => '▚',
//        (false, true, true, false) => '▞',
//
//        (false, true, true, true) => '▟',
//        (true, false, true, true) => '▙',
//        (true, true, false, true) => '▜',
//        (true, true, true, false) => '▛',

impl Node for Button {
    fn header(&self) -> &NodeHeader { &self.header }

    fn header_mut(&mut self) -> &mut NodeHeader { &mut self.header }

    fn paint(&self, w: &mut Canvas) {
        write!(w.writer, "{}", Background(242));
        write!(w.writer, "{}", Foreground(231));
        w.draw((0, 0), &iter::once('▛').chain(iter::repeat('▀').take(self.text.len())).chain(iter::once('▜')).collect::<String>());
        w.draw((0, 1), &format!("▌{}▐", self.text, ));
        w.draw((0, 2), &iter::once('▙').chain(iter::repeat('▄').take(self.text.len())).chain(iter::once('▟')).collect::<String>());
    }

    fn handle_event(&mut self, event: &Event) -> Option<NodeEvent> {
        match event {
            Event::MouseEvent(e) => {
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