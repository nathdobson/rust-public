use crate::gui::node::{Node, NodeImpl, NodeHeader};
use std::collections::{HashSet, BTreeSet};
use crate::screen::{Screen, LineSetting};
use crate::canvas::Canvas;
use crate::input::{Event, Mouse};
use std::process::Child;
use crate::gui::node::NodeExt;
use crate::input::MouseEvent;
use crate::gui::{InputEvent, OutputEvent};
use std::sync::Arc;
use crate::line::Table;

#[derive(Debug)]
pub struct Children {
    children: HashSet<Node>,
    mouse_down_focus: Option<Node>,
    mouse_motion_focus: Option<Node>,
}

impl Children {
    pub fn new(children: HashSet<Node>) -> Self {
        Children {
            children,
            mouse_down_focus: None,
            mouse_motion_focus: None,
        }
    }
}

pub trait Container: NodeImpl {
    fn children(&self) -> &Children;
    fn children_mut(&mut self) -> &mut Children;
    fn paint_children(&self, mut w: Canvas) {
        for node in self.children().children.iter() {
            let borrow = node.borrow_mut();
            borrow.paint(w.push_translate(borrow.position()))
        }
    }
    fn child_at(&self, position: (isize, isize)) -> Option<Node> {
        for node in self.children().children.iter() {
            let borrow = node.borrow();
            if node.borrow().bounds().contains(position) {
                return Some(node.clone());
            }
        }
        None
    }
    fn handle_children(&mut self, event: &InputEvent, output: &mut Vec<Arc<dyn OutputEvent>>) {
        match event {
            InputEvent::MouseEvent(mouse_event) => {
                let mut pass_event = |node: &Node| {
                    let mut node = node.borrow_mut();
                    let position =
                        (mouse_event.position.0 - node.position().0,
                         mouse_event.position.1 - node.position().1);
                    node.handle(
                        &InputEvent::MouseEvent(MouseEvent {
                            position: position,
                            ..*mouse_event
                        }), output);
                };

                if mouse_event.motion {
                    if let Some(mouse_focus) = self.children().mouse_motion_focus.as_ref() {
                        pass_event(mouse_focus);
                    }
                    if let Some(new) = self.child_at(mouse_event.position) {
                        if Some(&new) != self.children().mouse_motion_focus.as_ref() {
                            pass_event(&new);
                            self.children_mut().mouse_motion_focus = Some(new);
                        }
                    }
                } else {
                    let receiver = match mouse_event.mouse {
                        Mouse::Up => self.children().mouse_down_focus.clone(),
                        Mouse::Down(x) => {
                            self.children_mut().mouse_down_focus = self.child_at(mouse_event.position);
                            self.children().mouse_down_focus.clone()
                        }
                        _ => self.child_at(mouse_event.position),
                    };
                    if let Some(receiver) = receiver {
                        pass_event(&receiver)
                    }
                };
            }
            _ => {}
        }
    }
    fn size_children(&self) -> (isize, isize) {
        let iter = self.children().children.iter().map(|child| child.borrow().bounds());
        let w = iter.clone().map(|bound| bound.xs().end).max().unwrap_or(0);
        let h = iter.map(|bound| bound.ys().end).max().unwrap_or(0);
        (w, h)
    }
    fn line_setting_children(&self, y: isize) -> Option<LineSetting> {
        let line_settings: BTreeSet<LineSetting> =
            self.children().children.iter()
                .filter_map(|child| {
                    let borrow = child.borrow();
                    let ys = borrow.bounds().ys();
                    if ys.contains(&y) {
                        return borrow.line_setting(y - ys.start);
                    } else {
                        None
                    }
                })
                .collect();
        if line_settings.len() > 1 {
            eprintln!("Duplicate line setting {:?}", line_settings);
        }
        line_settings.iter().next().cloned()
    }
    fn check_dirty_children(&mut self) -> bool {
        let mut dirty = self.header_mut().check_dirty();
        for child in self.children().children.iter() {
            dirty |= child.borrow_mut().check_dirty();
        }
        return dirty;
    }
    fn insert(&mut self, node: Node) {
        if self.children_mut().children.insert(node) {
            self.header_mut().mark_dirty();
        }
    }
    fn remove(&mut self, node: &Node) {
        if self.children_mut().children.remove(node) {
            self.header_mut().mark_dirty();
        }
    }
    fn set_children(&mut self, children: HashSet<Node>) {
        if self.children_mut().children != children {
            self.children_mut().children = children;
            self.header_mut().mark_dirty();
        }
    }
}

#[derive(Debug)]
pub struct Panel {
    header: NodeHeader,
    children: Children,
    outline: Table,
}

impl Panel {
    pub fn new() -> Node<Panel> {
        Self::new_internal(|header| {
            Panel {
                header,
                children: Children::new(HashSet::new()),
                outline: Table::default(),
            }
        })
    }
    pub fn set_outline(&mut self, outline: Table) {
        self.outline = outline;
        self.header_mut().mark_dirty();
    }
}

impl Container for Panel {
    fn children(&self) -> &Children {
        &self.children
    }
    fn children_mut(&mut self) -> &mut Children {
        &mut self.children
    }
}

impl NodeImpl for Panel {
    fn header(&self) -> &NodeHeader {
        &self.header
    }
    fn header_mut(&mut self) -> &mut NodeHeader {
        &mut self.header
    }
    fn paint(&self, mut w: Canvas) {
        self.paint_children(w.push());
        self.outline.render_grid(w);
    }
    fn handle(&mut self, event: &InputEvent, output: &mut Vec<Arc<dyn OutputEvent>>) {
        self.handle_children(event, output);
    }
    fn size(&self) -> (isize, isize) {
        self.size_children()
    }
    fn line_setting(&self, y: isize) -> Option<LineSetting> {
        self.line_setting_children(y)
    }
    fn check_dirty(&mut self) -> bool {
        self.check_dirty_children()
    }
}