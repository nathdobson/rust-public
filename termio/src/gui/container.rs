use crate::gui::node::{Node, NodeHeader};
use util::rect::Rect;
use crate::canvas::Canvas;
use crate::gui::layout::Constraint;
use std::collections::HashMap;
use crate::screen::LineSetting;
use std::collections::hash_map::Entry;
use crate::gui::gui::{InputEvent, OutputEvent};
use crate::input::MouseEvent;

pub struct Container<T: ContainerImpl> {
    header: NodeHeader,
    line_settings: HashMap<isize, LineSetting>,
    inner: T,
    mouse_focus: bool,
}

impl<T: ContainerImpl> Container<T> {
    pub fn new(inner: T) -> Container<T> {
        Container {
            header: NodeHeader::new(),
            line_settings: HashMap::new(),
            inner,
            mouse_focus: false,
        }
    }
}

impl<T: ContainerImpl> Node for Container<T> {
    fn paint(&self, mut canvas: Canvas) {
        for child in self.inner.children() {
            child.paint(
                canvas
                    .push_bounds(*child.header().bounds())
                    .push_translate(child.header().position()));
        }
        self.inner.paint(canvas);
    }

    fn layout(&mut self, constraint: &Constraint) {
        let size = self.inner.layout(constraint);
        self.header_mut().set_size(size);
        self.line_settings.clear();
        for child in self.inner.children() {
            for y in 0..child.header().size().1 {
                if let Some(line_setting) = child.line_setting(y) {
                    self.line_settings
                        .entry(child.header().position().1 + y)
                        .and_modify(|old| *old = old.merge(line_setting))
                        .or_insert(line_setting);
                }
            }
        }
    }

    fn header(&self) -> &NodeHeader { &self.header }

    fn header_mut(&mut self) -> &mut NodeHeader { &mut self.header }

    fn check_dirty(&mut self) -> bool {
        let mut dirty = false;
        for child in self.inner.children_mut() {
            dirty |= child.check_dirty()
        }
        dirty |= self.header.check_dirty();
        dirty
    }

    fn line_setting(&self, row: isize) -> Option<LineSetting> {
        self.line_settings.get(&row).cloned()
    }

    fn handle(&mut self, event: &InputEvent, output: &mut Vec<OutputEvent>) {
        match event {
            InputEvent::MouseEvent { event, inside } => {
                if !self.mouse_focus && !*inside {
                    return;
                }
                self.mouse_focus = *inside;
                for child in self.inner.children_mut() {
                    let event = InputEvent::MouseEvent {
                        event: MouseEvent {
                            position: (event.position.0 - child.header().position().0,
                                       event.position.1 - child.header().position().1),
                            ..event.clone()
                        },
                        inside: child.header().bounds().contains(event.position),
                    };
                    child.handle(&event, output)
                }
            }
        }
    }
}

pub trait ContainerImpl {
    fn children(&self) -> Vec<&dyn Node>;
    fn children_mut(&mut self) -> Vec<&mut dyn Node>;
    fn layout(&mut self, constraint: &Constraint) -> (isize, isize);
    fn paint(&self, canvas: Canvas){}
}

