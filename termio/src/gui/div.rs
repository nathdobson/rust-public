use std::fmt::Debug;
use std::any::Any;
use util::any::Upcast;
use atomic_refcell::{AtomicRefCell, AtomicRefMut, AtomicRef};
use std::rc::Rc;
use util::shared::Shared;
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

pub trait DivImpl: 'static + Send + Sync + Upcast<dyn Any> + Debug + UpcastDiv {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout;
}

pub struct Div<T: ?Sized = dyn DivImpl> {
    this: DivWeak,
    parent: Option<DivWeak>,
    bounds: Rect,
    visible: bool,
    mouse_focus: bool,
    line_settings: HashMap<isize, LineSetting>,
    children: HashSet<DivWeak>,
    inner: T,
}

pub struct DivRc<T: ?Sized = dyn DivImpl>(Arc<AtomicRefCell<Div<T>>>);

pub struct DivWeak<T: ?Sized = dyn DivImpl>(Weak<AtomicRefCell<Div<T>>>);

pub type DivRef<'a, T> = AtomicRef<'a, Div<T>>;

pub type DivRefMut<'a, T> = AtomicRefMut<'a, Div<T>>;

impl<T: DivImpl> DivRc<T> {
    fn new(inner: T) -> Self {
        DivRc(Arc::new_cyclic(|this| {
            AtomicRefCell::new(Div {
                this: DivWeak(this.clone()).upcast_div(),
                parent: None,
                bounds: Rect::default(),
                visible: true,
                mouse_focus: false,
                line_settings: HashMap::new(),
                children: HashSet::new(),
                inner,
            })
        }))
    }
}

impl<T: DivImpl + ?Sized> DivRc<T> {
    pub fn read(&self) -> DivRef<T> {
        self.0.borrow()
    }
    pub fn write(&mut self) -> DivRefMut<T> {
        self.0.borrow_mut()
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
    fn upcast_div(self) -> DivRc {
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
    fn downgrade(&self) -> DivWeak<T> { DivWeak(Arc::downgrade(&self.0)) }
}

impl<T: DivImpl + ?Sized> DivWeak<T> {
    fn upcast_div(self) -> DivWeak {
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
    fn upgrade(&self) -> DivRc<T> { DivRc(self.0.upgrade().unwrap()) }
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

impl<T, U> CoerceUnsized<DivRc<U>> for DivRc<T> where T: Unsize<U> + ?Sized, U: ?Sized {}

impl<T, U> CoerceUnsized<DivWeak<U>> for DivWeak<T> where T: Unsize<U> + ?Sized, U: ?Sized {}