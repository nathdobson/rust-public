use util::rect::Rect;
use crate::canvas::Canvas;
use util::dynbag::Token;
use crate::input::MouseEvent;
use crate::screen::LineSetting;
use std::collections::HashMap;
use crate::gui::gui::{OutputEvent, InputEvent};
use crate::gui::layout::Constraint;

pub struct NodeHeader {
    bounds: Rect,
    dirty: bool,
}

impl NodeHeader {
    pub fn new() -> Self {
        NodeHeader {
            bounds: Rect::default(),
            dirty: true,
        }
    }
    pub fn check_dirty(&mut self) -> bool {
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
        self.bounds = Rect::from_position_size(self.bounds.position(), size);
    }
    pub fn position(&self) -> (isize, isize) {
        self.bounds.position()
    }
    pub fn set_position(&mut self, position: (isize, isize)) {
        self.bounds = Rect::from_position_size(position, self.bounds.size());
    }
}

pub trait Node {
    fn paint(&self, canvas: Canvas);
    fn handle(&mut self, event: &InputEvent, output: &mut Vec<OutputEvent>) {}
    fn layout(&mut self, constraint: &Constraint);
    fn header(&self) -> &NodeHeader;
    fn header_mut(&mut self) -> &mut NodeHeader;
    fn check_dirty(&mut self) -> bool { self.header_mut().check_dirty() }
    fn line_setting(&self, row: isize) -> Option<LineSetting> { Some(LineSetting::Normal) }
}