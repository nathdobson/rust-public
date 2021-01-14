use util::rect::Rect;
use crate::canvas::Canvas;
use util::dynbag::Token;
use crate::input::MouseEvent;
use crate::screen::LineSetting;
use std::collections::HashMap;
use crate::gui::gui::{OutputEvent, InputEvent};
use crate::gui::layout::Constraint;
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;

#[derive(Debug)]
pub struct Node<T: NodeImpl + ?Sized = dyn NodeImpl> {
    bounds: Rect,
    dirty: bool,
    visible: bool,
    inner: T,
}

impl<T: NodeImpl> Node<T> {
    pub fn new(inner: T) -> Self {
        Node {
            bounds: Rect::default(),
            dirty: true,
            visible: true,
            inner,
        }
    }
}

impl<T: NodeImpl + ?Sized> Node<T> {
    pub fn check_dirty_self(&mut self) -> bool {
        let dirty = self.dirty;
        self.dirty = false;
        dirty
    }
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    pub fn bounds(&self) -> &Rect {
        &self.bounds
    }
    pub fn size(&self) -> (isize, isize) {
        self.bounds.size()
    }
    pub fn set_size(&mut self, size: (isize, isize)) {
        assert!(size.0 >= 0);
        assert!(size.1 >= 0);
        self.bounds = Rect::from_position_size(self.bounds.position(), size);
    }
    pub fn position(&self) -> (isize, isize) {
        self.bounds.position()
    }
    pub fn set_position(&mut self, position: (isize, isize)) {
        self.bounds = Rect::from_position_size(position, self.bounds.size());
    }
    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.mark_dirty();
        }
    }
    pub fn visible(&self) -> bool {
        self.visible
    }
}

pub trait NodeImpl: Debug {
    fn paint(self: &Node<Self>, canvas: Canvas);
    fn handle(self: &mut Node<Self>, event: &InputEvent, output: &mut Vec<OutputEvent>) {}
    fn layout(self: &mut Node<Self>, constraint: &Constraint);
    fn check_dirty(self: &mut Node<Self>) -> bool { self.check_dirty_self() }
    fn line_setting(self: &Node<Self>, row: isize) -> Option<LineSetting> { Some(LineSetting::Normal) }
}

impl<T: NodeImpl + ?Sized> Deref for Node<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T: NodeImpl + ?Sized> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}