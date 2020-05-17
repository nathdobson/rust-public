use crate::canvas::Canvas;
use std::fmt;
use crate::input::Event;
use util::shared::{SharedMut, WkSharedMut, ObjectInner};
use std::ops::{Deref, DerefMut};
use util::rect::Rect;
use serde::export::fmt::Debug;
use std::any::Any;
use crate::screen::LineSetting;
use crate::gui::{InputEvent, OutputEvent};
use std::sync::Arc;
use backtrace::Backtrace;

pub type Node<T = dyn NodeImpl> = SharedMut<T>;

pub trait NodeImpl: Send + Sync + 'static + fmt::Debug {
    fn header(&self) -> &NodeHeader;
    fn header_mut(&mut self) -> &mut NodeHeader;
    fn paint(&self, w: Canvas);
    fn handle(&mut self, event: &InputEvent, output: &mut Vec<Arc<dyn OutputEvent>>);
    fn size(&self) -> (isize, isize);
    fn position(&self) -> (isize, isize) {
        self.header().position
    }
    fn bounds(&self) -> Rect {
        Rect::from_position_size(self.position(), self.size())
    }
    fn line_setting(&self, y: isize) -> Option<LineSetting> { Some(LineSetting::Normal) }
    fn check_dirty(&mut self) -> bool {
        self.header_mut().check_dirty()
    }
}

#[derive(Debug)]
pub struct NodeHeader {
    this_any: Option<WkSharedMut<dyn Any + 'static + Send + Sync>>,
    this_node: Option<WkSharedMut<dyn NodeImpl>>,
    pub position: (isize, isize),
    dirty: bool,
}

impl NodeHeader {
    pub fn this(&self) -> Node {
        self.this_node.as_ref().unwrap().upgrade().unwrap()
    }
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    pub fn check_dirty(&mut self) -> bool {
        let result = self.dirty;
        self.dirty = false;
        result
    }
}


pub trait NodeExt: NodeImpl {
    fn this(&self) -> Node<Self> where Self: Sized {
        self.header().this_any.as_ref().unwrap().upgrade().unwrap().downcast().unwrap()
    }
    fn new_internal(inner: impl FnOnce(NodeHeader) -> Self) -> Node<Self> where Self: Sized {
        let result = SharedMut::new(inner(NodeHeader {
            this_any: None,
            this_node: None,
            position: (0, 0),
            dirty: true,
        }));
        let this_any: SharedMut<dyn Any + 'static + Send + Sync> = result.clone();
        let this_node: Node = result.clone();
        result.borrow_mut().header_mut().this_any = Some(this_any.downgrade());
        result.borrow_mut().header_mut().this_node = Some(this_node.downgrade());
        result
    }
}

impl<T: NodeImpl> NodeExt for T {}

