use util::rect::Rect;
use crate::canvas::Canvas;
use util::dynbag::Token;
use crate::input::MouseEvent;
use crate::screen::LineSetting;
use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;
use atomic_refcell::AtomicRefCell;
use std::sync::Arc;
use by_address::ByAddress;
use std::lazy::OnceCell;
use util::shared::ObjectInner;
use util::any::{Upcast, AnyExt};
use std::any::Any;
use crate::gui::node::{Node, Children, ChildrenMut, NodeStrong};
use crate::gui::layout::{Constraint, Layout};
use crate::gui::gui::InputEvent;
use crate::gui::tree::{Tree};
use std::mem;
use std::raw::TraitObject;
use crate::gui::event::EventSender;


#[derive(Debug)]
pub struct View<T: ?Sized = dyn ViewImpl> {
    node: NodeStrong,
    bounds: Rect,
    visible: bool,
    mouse_focus: bool,
    line_settings: HashMap<isize, LineSetting>,
    inner: T,
}

pub trait UpcastView {
    fn upcast_view<'a>(self: &'a View<Self>) -> &'a View;
    fn upcast_view_mut<'a>(self: &'a mut View<Self>) -> &'a mut View;
}

impl<T: ViewImpl> UpcastView for T {
    fn upcast_view<'a>(self: &'a View<Self>) -> &'a View<dyn ViewImpl> { self }
    fn upcast_view_mut<'a>(self: &'a mut View<Self>) -> &'a mut View<dyn ViewImpl> { self }
}

pub trait ViewImpl: 'static + Send + Sync + Upcast<dyn Any> + Debug + UpcastView {
    fn layout_impl(self: &mut View<Self>, constraint: &Constraint) -> Layout;
    fn self_handle(self: &mut View<Self>, event: &InputEvent) -> bool { false }
    fn self_paint_below(self: &View<Self>, canvas: Canvas) {}
    fn self_paint_above(self: &View<Self>, canvas: Canvas) {}
}

impl<T: ?Sized> Deref for View<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: ?Sized + ViewImpl> DerefMut for View<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: ViewImpl> View<T> {
    pub fn new(node: NodeStrong<T>, inner: T) -> Self {
        View {
            node: node.upcast_node(),
            bounds: Rect::default(),
            visible: true,
            mouse_focus: false,
            line_settings: HashMap::new(),
            inner,
        }
    }
}

impl<T: ViewImpl + ?Sized> View<T> {
    pub fn downcast_view<T2: ViewImpl>(&self) -> &View<T2> {
        unsafe {
            let this: &View = self.upcast_view();
            let imp: &dyn ViewImpl = this.deref();
            let any: &dyn Any = imp.upcast();
            assert!(any.is::<T2>());
            let to: TraitObject = mem::transmute(this);
            let raw: *mut () = to.data;
            mem::transmute(raw)
        }
    }
    pub fn downcast_view_mut<T2: ViewImpl>(&mut self) -> &mut View<T2> {
        unsafe {
            let this: &mut View = self.upcast_view_mut();
            let imp: &mut dyn ViewImpl = this.deref_mut();
            let any: &mut dyn Any = imp.upcast_mut();
            assert!(any.is::<T2>());
            let to: TraitObject = mem::transmute(this);
            let raw: *mut () = to.data;
            mem::transmute(raw)
        }
    }
    pub fn node_strong(&self) -> &NodeStrong {
        &self.node
    }
    pub fn node(&self) -> Node<T> {
        self.node.downgrade().downcast_node()
    }
    pub fn mark_dirty(&self) {
        self.node.context().mark_dirty()
    }
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
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
    pub fn line_setting(&self, row: isize) -> Option<LineSetting> {
        self.line_settings.get(&row).cloned()
    }
    pub fn set_line_settings(&mut self, line_settings: HashMap<isize, LineSetting>) {
        self.line_settings = line_settings;
    }
    pub fn mouse_focus(&self) -> bool {
        self.mouse_focus
    }
    pub fn set_mouse_focus(&mut self, mouse_focus: bool) {
        self.mouse_focus = mouse_focus;
    }
    pub fn context(&self) -> &Tree {
        self.node.context()
    }
    pub fn event_sender(&self) -> &EventSender {
        self.context().event_sender()
    }

    // fn mark_dirty(&mut self) { self.node().mark_dirty(); }
    // fn bounds(&self) -> Rect { self.node().bounds() }
    // fn set_bounds(&mut self, bounds: Rect) { self.node_mut().set_bounds(bounds) }
    pub fn size(&self) -> (isize, isize) { self.bounds().size() }
    pub fn set_size(&mut self, size: (isize, isize)) {
        assert!(size.0 >= 0);
        assert!(size.1 >= 0);
        self.set_bounds(Rect::from_position_size(self.position(), size));
    }
    pub fn position(&self) -> (isize, isize) { self.bounds().position() }
    pub fn set_position(&mut self, position: (isize, isize)) {
        self.set_bounds(Rect::from_position_size(position, self.bounds().size()));
    }
    // fn set_visible(&mut self, visible: bool) {
    //     self.node_mut().set_visible(visible);
    // }
    // fn visible(&self) -> bool {
    //     self.node().visible()
    // }
    // fn line_setting(&self, row: isize) -> Option<LineSetting> {
    //     self.node().line_setting(row)
    // }
    pub fn children<'a>(&'a self) -> Children<'a> {
        Children::new(self.upcast_view())
    }
    pub fn children_mut<'a>(&'a self) -> ChildrenMut {
        ChildrenMut::new(self.upcast_view())
    }
    pub fn layout(&mut self, constraint: &Constraint) {
        let layout = self.layout_impl(constraint);
        self.set_size(layout.size);
        let mut line_settings = layout.line_settings;
        for child in self.children().into_iter() {
            let child: &View = child;
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
        self.set_line_settings(line_settings);
    }

    pub fn paint(&self, mut canvas: Canvas) {
        eprintln!("Printing {:?}", self);
        self.self_paint_below(canvas.push());
        for child in self.children().into_iter() {
            if child.visible() {
                child.paint(
                    canvas
                        .push_bounds(child.bounds())
                        .push_translate(child.position()));
            }
        }
        self.self_paint_above(canvas.push());
    }

    pub fn handle(&mut self, event: &InputEvent) {
        if self.self_handle(event) {
            return;
        }
        match event {
            InputEvent::MouseEvent { event, inside } => {
                if !self.mouse_focus() && !*inside {
                    return;
                }
                self.set_mouse_focus(*inside);
                for child in self.children_mut().into_iterable().into_iter() {
                    let child = child(self.upcast_view_mut());
                    if child.visible() {
                        let event = InputEvent::MouseEvent {
                            event: MouseEvent {
                                position: (event.position.0 - child.position().0,
                                           event.position.1 - child.position().1),
                                ..event.clone()
                            },
                            inside: child.bounds().contains(event.position),
                        };
                        child.handle(&event)
                    }
                }
            }
        }
    }
}

