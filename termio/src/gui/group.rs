use crate::gui::node::{Node};
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
    mouse_focus: bool,
    inner: G,
}

pub trait GroupImpl: Sized + Debug {
    fn children<'a>(this: &'a Node<Group<Self>>) -> Vec<&'a Node>;
    fn children_mut<'a>(this: &'a mut Node<Group<Self>>) -> Vec<&'a mut Node>;
    fn group_paint_below(this: &Node<Group<Self>>, canvas: Canvas) {}
    fn group_paint_above(this: &Node<Group<Self>>, canvas: Canvas) {}
    fn group_layout_self(this: &mut Node<Group<Self>>, constraint: &Constraint) -> (isize, isize);
}

impl<G: GroupImpl> Deref for Group<G> {
    type Target = G;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<G: GroupImpl> DerefMut for Group<G> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl<G: GroupImpl> Group<G> {
    pub fn new(inner: G) -> Node<Self> {
        Node::new(Group {
            line_settings: HashMap::new(),
            mouse_focus: false,
            inner,
        })
    }
}

impl<T: GroupImpl> NodeImpl for Group<T> {
    fn layout(self: &mut Node<Self>, constraint: &Constraint) {
        let size = T::group_layout_self(self, constraint);
        self.set_size(size);
        let mut line_settings = HashMap::<isize, LineSetting>::new();
        for child in T::children(self) {
            if child.visible() {
                for y in 0..child.size().1 {
                    if let Some(line_setting) = child.line_setting(y) {
                        line_settings.entry(child.position().1 + y)
                            .and_modify(|old| *old = old.merge(line_setting, &y))
                            .or_insert(line_setting);
                    }
                }
            }
        }
        self.line_settings = line_settings;
    }
    fn paint(self: &Node<Self>, mut canvas: Canvas) {
        T::group_paint_below(self, canvas.push());
        for child in T::children(self) {
            if child.visible() {
                child.paint(
                    canvas
                        .push_bounds(*child.bounds())
                        .push_translate(child.position()));
            }
        }
        T::group_paint_above(self, canvas.push());
    }
    fn check_dirty(self: &mut Node<Self>) -> bool {
        let mut dirty = false;
        for child in T::children_mut(self) {
            dirty |= child.check_dirty()
        }
        dirty |= self.check_dirty_self();
        dirty
    }
    fn line_setting(self: &Node<Self>, row: isize) -> Option<LineSetting> {
        self.line_settings.get(&row).cloned()
    }
    fn handle(self: &mut Node<Self>, event: &InputEvent, output: &mut Vec<OutputEvent>) {
        match event {
            InputEvent::MouseEvent { event, inside } => {
                if !self.mouse_focus && !*inside {
                    return;
                }
                self.mouse_focus = *inside;
                for child in T::children_mut(self) {
                    if child.visible() {
                        let event = InputEvent::MouseEvent {
                            event: MouseEvent {
                                position: (event.position.0 - child.position().0,
                                           event.position.1 - child.position().1),
                                ..event.clone()
                            },
                            inside: child.bounds().contains(event.position),
                        };
                        child.handle(&event, output)
                    }
                }
            }
            InputEvent::TimeEvent { when } => {
                // TODO filter
                for child in T::children_mut(self) {
                    child.handle(event, output)
                }
            }
        }
    }
}
