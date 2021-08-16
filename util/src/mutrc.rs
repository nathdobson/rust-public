#![allow(deprecated)]

use std::sync::{Arc, Weak, Mutex};
use crate::atomic_refcell::AtomicRefCell;
use std::{mem, hash, fmt};
use std::any::Any;
use std::cmp::Ordering;
use std::ops::CoerceUnsized;
use std::io::Write;
use std::marker::Unsize;
use std::fmt::Formatter;
use std::hash::Hasher;
use crate::atomic_refcell::{AtomicRef, AtomicRefMut};
use crate::any::Pointy;

pub struct MutRc<T: ?Sized>(Arc<AtomicRefCell<T>>);

pub struct MutWeak<T: ?Sized>(Weak<AtomicRefCell<T>>);

pub type ReadGuard<'a, T> = AtomicRef<'a, T>;

pub type WriteGuard<'a, T> = AtomicRefMut<'a, T>;

impl<T: ?Sized> MutRc<T> {
    pub fn new_cyclic(f: impl FnOnce(&MutWeak<T>) -> T) -> MutRc<T> where T: Sized {
        MutRc(Arc::new_cyclic(
            |w|
                AtomicRefCell::new(f(&MutWeak(w.clone())))
        ))
    }
    pub fn new(x: T) -> Self where T: Sized {
        MutRc(Arc::new(AtomicRefCell::new(x)))
    }
    pub fn into_inner(self) -> Result<T, Self> where T: Sized {
        Ok(Arc::try_unwrap(self.0).map_err(MutRc)?.into_inner())
    }
    pub fn read(&self) -> ReadGuard<T> {
        self.0.borrow()
    }
    pub fn write(&mut self) -> WriteGuard<T> { self.0.borrow_mut() }
    pub fn borrow_mut(&self) -> WriteGuard<T> { self.0.borrow_mut() }
    pub fn downgrade(&self) -> MutWeak<T> {
        MutWeak(Arc::downgrade(&self.0))
    }
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { mem::transmute_copy::<Self, *const u8>(self) }
    }
    pub fn as_inner_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }
    pub fn into_raw(this: Self) -> *const AtomicRefCell<T> {
        Arc::into_raw(this.0)
    }
    pub unsafe fn from_raw(raw: *const AtomicRefCell<T>) -> Self {
        MutRc(Arc::from_raw(raw))
    }
}

impl<T: ?Sized> MutWeak<T> {
    pub fn upgrade(&self) -> Option<MutRc<T>> {
        self.0.upgrade().map(MutRc)
    }
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { mem::transmute_copy::<Self, *const u8>(self) }
    }
    pub fn as_inner_ptr(&self) -> *mut T { unsafe { AtomicRefCell::raw_get(self.0.as_ptr()) } }
    pub fn into_raw(self) -> *const AtomicRefCell<T> {
        self.0.into_raw()
    }
    pub unsafe fn from_raw(raw: *const AtomicRefCell<T>) -> Self {
        MutWeak(Weak::from_raw(raw))
    }
}

impl MutRc<dyn Any + 'static + Send + Sync> {
    pub fn downcast<T: Any>(self) -> Result<MutRc<T>, Self> {
        unsafe {
            if self.read().is::<T>() {
                let raw: raw::TraitObject = mem::transmute(self);
                Ok(mem::transmute(raw.data))
            } else {
                Err(self)
            }
        }
    }
}

impl<T: ?Sized> Eq for MutRc<T> {}

impl<T: ?Sized> Eq for MutWeak<T> {}

impl<T: ?Sized> PartialEq for MutRc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl<T: ?Sized> PartialEq for MutWeak<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl<T: ?Sized> PartialOrd for MutRc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

impl<T: ?Sized> PartialOrd for MutWeak<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

impl<T: ?Sized> Ord for MutRc<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

impl<T: ?Sized> Ord for MutWeak<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

impl<T: ?Sized> hash::Hash for MutRc<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state)
    }
}

impl<T: ?Sized> hash::Hash for MutWeak<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state)
    }
}

impl<T: ?Sized> Clone for MutRc<T> {
    fn clone(&self) -> Self {
        MutRc(self.0.clone())
    }
}

impl<T: ?Sized> Clone for MutWeak<T> {
    fn clone(&self) -> Self {
        MutWeak(self.0.clone())
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MutRc<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} -> ", self.as_ptr())?;
        self.read().fmt(f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MutWeak<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_ptr())
    }
}

impl<T, U> CoerceUnsized<MutRc<U>> for MutRc<T> where T: ?Sized + Unsize<U>, U: ?Sized {}

impl<T, U> CoerceUnsized<MutWeak<U>> for MutWeak<T> where T: ?Sized + Unsize<U>, U: ?Sized {}

unsafe impl<T: ?Sized> Pointy for MutRc<T> { type Target = T; }

unsafe impl<T: ?Sized> Pointy for MutWeak<T> { type Target = T; }
