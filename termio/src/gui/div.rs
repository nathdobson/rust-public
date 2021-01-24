use std::fmt::Debug;
use std::any::Any;
use util::any::Upcast;
use util::atomic_refcell::{AtomicRefCell, AtomicRefMut, AtomicRef};
use std::rc::Rc;
use util::shared::{Shared, ObjectInner};
use util::rect::Rect;
use std::collections::{HashMap, HashSet};
use crate::screen::LineSetting;
use std::sync::{Arc, Weak};
use crate::gui::layout::{Constraint, Layout};
use std::ops::{Deref, DerefMut, CoerceUnsized};
use std::cmp::Ordering;
use std::marker::Unsize;
use sealed::UpcastDiv;
use std::raw::TraitObject;
use std::mem;
use crate::gui::gui::InputEvent;
use crate::canvas::Canvas;
use crate::input::MouseEvent;
use crate::gui::tree::{Tree, Dirty};
use crate::gui::event::{EventSender, GuiEvent, SharedGuiEvent};
use std::hash::{Hash, Hasher};
use std::ptr::{null, null_mut};
use util::mutrc::{MutRc, MutWeak};
use std::borrow::BorrowMut;

pub trait DivImpl: 'static + Send + Sync + Upcast<dyn Any> + Debug + UpcastDiv {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout;
    fn self_handle(self: &mut Div<Self>, event: &InputEvent) -> bool { false }
    fn self_paint_below(self: &Div<Self>, canvas: Canvas) {}
    fn self_paint_above(self: &Div<Self>, canvas: Canvas) {}
}

#[derive(Debug)]
pub struct Div<T: ?Sized = dyn DivImpl> {
    this: DivWeak,
    parent: Option<DivWeak>,
    tree: Tree,
    bounds: Rect,
    visible: bool,
    mouse_focus: bool,
    line_settings: HashMap<isize, LineSetting>,
    children: HashSet<DivRc>,
    inner: T,
}

#[derive(Debug)]
pub struct DivRc<T: ?Sized = dyn DivImpl>(MutRc<Div<T>>);

#[derive(Debug)]
pub struct DivWeak<T: ?Sized = dyn DivImpl>(MutWeak<Div<T>>);

pub type DivRef<'a, T> = AtomicRef<'a, Div<T>>;

pub type DivRefMut<'a, T> = AtomicRefMut<'a, Div<T>>;

impl<T: DivImpl> DivRc<T> {
    pub fn new(tree: Tree, inner: T) -> Self {
        Self::new_cyclic(tree, |_| inner)
    }
    pub fn new_cyclic(tree: Tree, inner: impl FnOnce(DivWeak<T>) -> T) -> Self {
        DivRc(MutRc::new_cyclic(|this| {
            let this = DivWeak(this.clone());
            Div {
                this: this.clone().upcast_div(),
                parent: None,
                tree,
                bounds: Rect::default(),
                visible: true,
                mouse_focus: false,
                line_settings: HashMap::new(),
                children: HashSet::new(),
                inner: inner(this),
            }
        }))
    }
}

impl<T: DivImpl + ?Sized> DivRc<T> {
    pub fn read(&self) -> DivRef<T> { self.0.read() }
    pub fn write(&mut self) -> DivRefMut<T> { self.0.write() }
    pub fn borrow_mut(&self) -> DivRefMut<T> { self.0.borrow_mut() }
}

impl<T: DivImpl + ?Sized> Div<T> {
    pub fn div_rc(&self) -> DivRc<T> { self.this.upgrade().unwrap().downcast_div() }
    pub fn div_weak(&self) -> DivWeak<T> {
        self.this.clone().downcast_div()
    }
    pub fn mark_dirty(&mut self, dirty: Dirty) { self.tree_mut().mark_dirty(dirty) }
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn set_bounds(&mut self, bounds: Rect) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.mark_dirty(Dirty::Paint);
        }
    }
    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.mark_dirty(Dirty::Paint);
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
    pub fn tree(&self) -> &Tree { &self.tree }
    pub fn tree_mut(&mut self) -> &mut Tree { &mut self.tree }
    pub fn event_sender(&self) -> &EventSender { self.tree().event_sender() }

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

    pub fn children<'a>(&'a self) -> impl 'a + Iterator<Item=&'a DivRc> {
        self.children.iter()
    }
    pub fn children_mut<'a>(&'a mut self) -> impl 'a + Iterator<Item=DivRc> {
        self.children.iter().cloned()
    }
    pub fn add(&mut self, mut child: DivRc) {
        assert!(child.write().parent.replace(self.this.clone()).is_none());
        assert!(self.children.insert(child));
    }
    pub fn remove(&mut self, child: &DivRc) {
        assert!(child.borrow_mut().parent.take().is_some());
        assert!(self.children.remove(&child));
    }
    pub fn new_event(&self, f: impl 'static + Send + Sync + FnOnce(&mut Self)) -> GuiEvent {
        self.div_weak().new_event(f)
    }
    pub fn new_shared_event(&self, f: impl 'static + Send + Sync + Fn(&mut Self)) -> SharedGuiEvent {
        self.div_weak().new_shared_event(f)
    }

    pub fn layout(&mut self, constraint: &Constraint) {
        let layout = self.layout_impl(constraint);
        self.set_size(layout.size);
        let mut line_settings = layout.line_settings;
        for child in self.children() {
            let child = child.read();
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
        self.self_paint_below(canvas.push());
        for child in self.children() {
            let child = child.read();
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
                for mut child in self.children_mut() {
                    let mut child = child.write();
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
            _ => {}
        }
    }
}

impl<T: DivImpl + ?Sized> DivWeak<T> {
    pub fn new_event(&self, f: impl 'static + Send + Sync + FnOnce(&mut Div<T>)) -> GuiEvent {
        let this = self.clone();
        GuiEvent::new(move || {
            if let Some(mut this) = this.upgrade() {
                f(&mut *this.write())
            }
        })
    }
    pub fn new_shared_event(&self, f: impl 'static + Send + Sync + Fn(&mut Div<T>)) -> SharedGuiEvent {
        let this = self.clone();
        SharedGuiEvent::new(move || {
            if let Some(mut this) = this.upgrade() {
                f(&mut *this.write())
            }
        })
    }
}

impl<T: DivImpl + ?Sized> DivRc<T> {
    pub fn new_event(&self, f: impl 'static + Send + Sync + FnOnce(&mut Div<T>)) -> GuiEvent {
        self.downgrade().new_event(f)
    }
    pub fn new_shared_event(&self, f: impl 'static + Send + Sync + Fn(&mut Div<T>)) -> SharedGuiEvent {
        self.downgrade().new_shared_event(f)
    }
}

impl<T: DivImpl + ?Sized> Div<T> {
    pub fn upcast_div(&self) -> &Div { self.upcast_div_impl() }
    pub fn upcast_div_mut(&mut self) -> &mut Div { self.upcast_div_mut_impl() }
    pub fn downcast_div<T2: DivImpl>(&self) -> &Div<T2> {
        unsafe {
            let this: &Div = self.upcast_div();
            let imp: &dyn DivImpl = this.deref();
            let any: &dyn Any = imp.upcast();
            assert!(any.is::<T2>());
            let to: TraitObject = mem::transmute(this);
            let raw: *mut () = to.data;
            mem::transmute(raw)
        }
    }
    pub fn downcast_div_mut<T2: DivImpl>(&mut self) -> &mut Div<T2> {
        unsafe {
            let this: &mut Div = self.upcast_div_mut();
            let imp: &mut dyn DivImpl = this.deref_mut();
            let any: &mut dyn Any = imp.upcast_mut();
            assert!(any.is::<T2>());
            let to: TraitObject = mem::transmute(this);
            let raw: *mut () = to.data;
            mem::transmute(raw)
        }
    }
}

mod sealed {
    use crate::gui::div::{DivImpl, Div};

    pub trait UpcastDiv {
        fn upcast_div_impl<'a>(self: &'a Div<Self>) -> &'a Div;
        fn upcast_div_mut_impl<'a>(self: &'a mut Div<Self>) -> &'a mut Div;
    }

    impl<T: DivImpl> UpcastDiv for T {
        fn upcast_div_impl<'a>(self: &'a Div<Self>) -> &'a Div<dyn DivImpl> { self }
        fn upcast_div_mut_impl<'a>(self: &'a mut Div<Self>) -> &'a mut Div<dyn DivImpl> { self }
    }
}

impl<T: DivImpl + ?Sized> DivRc<T> {
    pub fn upcast_div(self) -> DivRc {
        trait DivRcImpl {
            fn upcast_div_impl(self) -> DivRc;
        }
        impl<T: DivImpl + ?Sized> DivRcImpl for DivRc<T> {
            default fn upcast_div_impl(self) -> DivRc { unimplemented!() }
        }
        impl DivRcImpl for DivRc {
            fn upcast_div_impl(self) -> DivRc { self }
        }
        impl<T2: DivImpl> DivRcImpl for DivRc<T2> {
            fn upcast_div_impl(self) -> DivRc { self }
        }
        self.upcast_div_impl()
    }
    pub fn downcast_div<T2: DivImpl + ?Sized>(self) -> DivRc<T2> {
        trait DivRcImpl {
            fn downcast_div_impl(this: DivRc) -> Self;
        }
        impl<T: DivImpl + ?Sized> DivRcImpl for DivRc<T> {
            default fn downcast_div_impl(this: DivRc) -> Self { unimplemented!() }
        }
        impl DivRcImpl for DivRc {
            fn downcast_div_impl(this: DivRc) -> Self { this }
        }
        impl<T: DivImpl> DivRcImpl for DivRc<T> {
            fn downcast_div_impl(this: DivRc) -> Self {
                unsafe {
                    let null: *mut T = null_mut();
                    let null_div: *mut dyn DivImpl = null;
                    let expected_trait_object: TraitObject = mem::transmute(null_div);
                    let this_ptr: *const AtomicRefCell<Div<dyn DivImpl>> = MutRc::into_raw(this.0);
                    let trait_object: TraitObject = mem::transmute(this_ptr);
                    assert_eq!(expected_trait_object.vtable, trait_object.vtable);
                    let this_ptr_cast: *const AtomicRefCell<Div<T>> = mem::transmute(trait_object.data);
                    DivRc(MutRc::from_raw(this_ptr_cast))
                }
            }
        }
        DivRc::<T2>::downcast_div_impl(self.upcast_div())
    }
    pub fn downgrade(&self) -> DivWeak<T> { DivWeak(MutRc::downgrade(&self.0)) }
}

impl<T: DivImpl + ?Sized> DivWeak<T> {
    pub fn upcast_div(self) -> DivWeak {
        trait DivWeakImpl {
            fn upcast_div_impl(self) -> DivWeak;
        }
        impl<T: DivImpl + ?Sized> DivWeakImpl for DivWeak<T> {
            default fn upcast_div_impl(self) -> DivWeak<dyn DivImpl> { unimplemented!() }
        }
        impl DivWeakImpl for DivWeak {
            fn upcast_div_impl(self) -> DivWeak<dyn DivImpl> { self }
        }
        impl<T: DivImpl> DivWeakImpl for DivWeak<T> {
            fn upcast_div_impl(self) -> DivWeak<dyn DivImpl> { self }
        }
        self.upcast_div_impl()
    }
    pub fn downcast_div<T2: DivImpl + ?Sized>(self) -> DivWeak<T2> {
        trait DivWeakImpl {
            fn downcast_div_impl(this: DivWeak) -> Self;
        }
        impl<T: DivImpl + ?Sized> DivWeakImpl for DivWeak<T> {
            default fn downcast_div_impl(this: DivWeak) -> Self { unimplemented!() }
        }
        impl DivWeakImpl for DivWeak {
            fn downcast_div_impl(this: DivWeak) -> Self { this }
        }
        impl<T: DivImpl> DivWeakImpl for DivWeak<T> {
            fn downcast_div_impl(this: DivWeak) -> Self {
                unsafe {
                    let null: *mut T = null_mut();
                    let null_div: *mut dyn DivImpl = null;
                    let expected_trait_object: TraitObject = mem::transmute(null_div);
                    let this_ptr: *const AtomicRefCell<Div<dyn DivImpl>> = this.0.into_raw();
                    let trait_object: TraitObject = mem::transmute(this_ptr);
                    assert_eq!(expected_trait_object.vtable, trait_object.vtable);
                    let this_ptr_cast: *const AtomicRefCell<Div<T>> = mem::transmute(trait_object.data);
                    DivWeak(MutWeak::from_raw(this_ptr_cast))
                }
            }
        }
        DivWeak::<T2>::downcast_div_impl(self.upcast_div())
    }
    pub fn upgrade(&self) -> Option<DivRc<T>> { Some(DivRc(self.0.upgrade()?)) }
}

impl<T: ?Sized> Eq for DivRc<T> {}

impl<T: ?Sized> PartialEq for DivRc<T> {
    fn eq(&self, other: &Self) -> bool { self.0.as_ptr().eq(&other.0.as_ptr()) }
}

impl<T: ?Sized> PartialOrd for DivRc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.as_ptr().partial_cmp(&other.0.as_ptr())
    }
}

impl<T: ?Sized> Ord for DivRc<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.as_ptr().cmp(&other.0.as_ptr())
    }
}

impl<T: ?Sized> Eq for DivWeak<T> {}

impl<T: ?Sized> PartialEq for DivWeak<T> {
    fn eq(&self, other: &Self) -> bool { self.0.as_ptr().eq(&other.0.as_ptr()) }
}

impl<T: ?Sized> PartialOrd for DivWeak<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.as_ptr().partial_cmp(&other.0.as_ptr())
    }
}

impl<T: ?Sized> Ord for DivWeak<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.as_ptr().cmp(&other.0.as_ptr())
    }
}

impl<T: ?Sized> Deref for Div<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T: ?Sized> DerefMut for Div<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl<T: ?Sized> Clone for DivRc<T> {
    fn clone(&self) -> Self { DivRc(self.0.clone()) }
}

impl<T: ?Sized> Clone for DivWeak<T> {
    fn clone(&self) -> Self { DivWeak(self.0.clone()) }
}

impl<T: ?Sized> Hash for DivRc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state)
    }
}

impl<T: ?Sized> Hash for DivWeak<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state)
    }
}

impl<T, U> CoerceUnsized<DivRc<U>> for DivRc<T> where T: Unsize<U> + ?Sized, U: ?Sized {}

impl<T, U> CoerceUnsized<DivWeak<U>> for DivWeak<T> where T: Unsize<U> + ?Sized, U: ?Sized {}