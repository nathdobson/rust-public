use util::rect::Rect;
use crate::canvas::Canvas;
use util::dynbag::Token;
use crate::input::MouseEvent;
use crate::screen::LineSetting;
use std::collections::{HashMap, HashSet};
use crate::gui::gui::{OutputEvent, InputEvent, Context};
use crate::gui::layout::Constraint;
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;
use atomic_refcell::AtomicRefCell;
use std::sync::Arc;
use by_address::ByAddress;
use std::lazy::OnceCell;
use util::shared::ObjectInner;
use util::any::{Upcast, AnyExt};
use std::any::Any;

struct NodeParent {
    id: NodeId,
    get_ref: Box<dyn Fn(&Node) -> &Node>,
    get_mut: Box<dyn Fn(&mut Node) -> &mut Node>,
}

#[derive(Debug)]
struct NodeIdInner {
    parent: OnceCell<Option<NodeParent>>,
    context: Context,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct NodeId(ByAddress<Arc<NodeIdInner>>);

#[derive(Debug)]
pub struct Node<T: NodeImpl + ?Sized = dyn NodeImpl> {
    bounds: Rect,
    id: NodeId,
    visible: bool,
    mouse_focus: bool,
    children: HashSet<NodeId>,
    inner: T,
}

pub trait NodeImpl: Debug + Send + Sync + Upcast<dyn Any> {
    fn self_handle(self: &mut Node<Self>, event: &InputEvent, output: &mut Vec<OutputEvent>) {}
    fn layout_impl(self: &mut Node<Self>, constraint: &Constraint) -> (isize, isize);
    fn self_paint_below(self: &Node<Self>, canvas: Canvas) {}
    fn self_paint_above(self: &Node<Self>, canvas: Canvas) {}
    fn self_line_setting(self: &Node<Self>, row: isize) -> Option<LineSetting> { Some(LineSetting::Normal) }
}

impl NodeId {
    pub fn new() -> Self {
        NodeId(ByAddress(Arc::new(OnceCell::new())))
    }
}

impl<T: NodeImpl> Node<T> {
    pub fn new(inner: T) -> Self {
        Node {
            bounds: Rect::default(),
            id,
            visible: true,
            mouse_focus: false,
            children: HashSet::new(),
            inner,
        }
    }
}

impl<T: NodeImpl + ?Sized> Node<T> {
    pub fn mark_dirty(&mut self) {
        self.id.0.context.mark_dirty();
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
    pub fn add_child(&mut self,
                     child: NodeId,
                     get_ref: impl Fn(&Self) -> &Node,
                     get_mut: impl Fn(&mut Self) -> &mut Node) {
        self.children.insert(child);
        child.0.set(NodeIdInner {
            parent: self.id.clone(),
            get_ref: Box::new(|this|
                get_ref(this.inner.downcast_ref_result().unwrap())
            ),
            get_mut: Box::new(|this|
                get_mut(this.inner.downcast_mut_result().unwrap())
            ),
        });
    }

    pub fn layout(&mut self, constraint: &Constraint) {
        let size = T::layout_impl(self, constraint);
        self.set_size(size);
        let mut line_settings = HashMap::<isize, LineSetting>::new();
        for child in self.children() {
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
        for y in 0..self.size().1 {
            if let Some(line_setting) = self.self_line_setting(y) {
                line_settings.entry(y)
                    .and_modify(|old| *old = old.merge(line_setting, &y))
                    .or_insert(line_setting);
            }
        }
        self.line_settings = line_settings;
    }
    fn paint(&mut self, mut canvas: Canvas) {
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
    fn line_setting(&mut self, row: isize) -> Option<LineSetting> {
        self.line_settings.get(&row).cloned()
    }
    fn handle(&mut self, event: &InputEvent, output: &mut Vec<OutputEvent>) {
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

impl<T: NodeImpl + ?Sized> Deref for Node<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T: NodeImpl + ?Sized> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}
