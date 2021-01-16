use crate::gui::node::{Node, NodeId};
use util::rect::Rect;
use crate::canvas::Canvas;
use crate::gui::layout::Constraint;
use std::collections::HashMap;
use crate::screen::LineSetting;
use std::collections::hash_map::Entry;
use crate::gui::gui::{InputEvent, OutputEvent};
use crate::input::MouseEvent;
use std::ops::{Deref, DerefMut};
use crate::gui::node::NodeImpl;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Group<G: GroupImpl> {
    line_settings: HashMap<isize, LineSetting>,
    inner: G,
}

pub trait GroupImpl: Sized + Debug + Send + Sync {
}

impl<G: GroupImpl> Deref for Group<G> {
    type Target = G;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<G: GroupImpl> DerefMut for Group<G> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl<G: GroupImpl> Group<G> {
    pub fn new(id: NodeId, inner: G) -> Node<Self> {
        Node::new(id, Group {
            line_settings: HashMap::new(),
            mouse_focus: false,
            inner,
        })
    }
}
