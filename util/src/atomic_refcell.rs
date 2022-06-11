use std::backtrace::Backtrace;
use std::cell::UnsafeCell;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::{fmt, mem};

use by_address::ByAddress;

struct State {
    readers: HashSet<ByAddress<Arc<Backtrace>>>,
    writer: Option<Backtrace>,
}

pub struct AtomicRefCell<T: ?Sized> {
    state: Mutex<State>,
    inner: UnsafeCell<T>,
}

pub struct AtomicRef<'a, T: ?Sized> {
    cell: &'a AtomicRefCell<T>,
    bt: ByAddress<Arc<Backtrace>>,
}

pub struct AtomicRefMut<'a, T: ?Sized> {
    cell: &'a AtomicRefCell<T>,
}

impl<T: ?Sized> AtomicRefCell<T> {
    pub fn new(inner: T) -> Self
    where
        T: Sized,
    {
        AtomicRefCell {
            state: Mutex::new(State {
                readers: HashSet::new(),
                writer: None,
            }),
            inner: UnsafeCell::new(inner),
        }
    }
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.inner.into_inner()
    }
    pub fn borrow(&self) -> AtomicRef<T> {
        let mut lock = self.state.lock().unwrap();
        if let Some(writer) = &lock.writer {
            eprintln!("During borrow_mut():");
            eprintln!("{}", writer);
            mem::drop(lock);
            panic!("Called borrow():");
        }
        let bt = ByAddress(Arc::new(Backtrace::capture()));
        lock.readers.insert(bt.clone());
        AtomicRef { cell: self, bt }
    }
    pub fn borrow_mut(&self) -> AtomicRefMut<T> {
        let mut lock = self.state.lock().unwrap();
        if let Some(writer) = &lock.writer {
            eprintln!("During borrow_mut():");
            eprintln!("{}", writer);
            mem::drop(lock);
            panic!("Called borrow_mut():");
        }
        if !lock.readers.is_empty() {
            for reader in lock.readers.iter() {
                eprintln!("During borrow():");
                eprintln!("{}", reader.0);
            }
            mem::drop(lock);
            panic!("Called borrow_mut():");
        }
        lock.writer = Some(Backtrace::capture());
        AtomicRefMut { cell: self }
    }
    pub fn as_ptr(&self) -> *mut T { self.inner.get() }
    pub unsafe fn raw_get(this: *const AtomicRefCell<T>) -> *mut T {
        UnsafeCell::raw_get(&raw const (*this).inner)
    }
}

impl<'a, T: ?Sized> Drop for AtomicRef<'a, T> {
    fn drop(&mut self) { self.cell.state.lock().unwrap().readers.remove(&self.bt); }
}

impl<'a, T: ?Sized> Drop for AtomicRefMut<'a, T> {
    fn drop(&mut self) { self.cell.state.lock().unwrap().writer = None; }
}

impl<'a, T: ?Sized> Deref for AtomicRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { unsafe { &*self.cell.inner.get() } }
}

impl<'a, T: ?Sized> Deref for AtomicRefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { unsafe { &*self.cell.inner.get() } }
}

impl<'a, T: ?Sized> DerefMut for AtomicRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target { unsafe { &mut *self.cell.inner.get() } }
}

unsafe impl<T: ?Sized + Send> Send for AtomicRefCell<T> {}

unsafe impl<T: ?Sized + Send + Sync> Sync for AtomicRefCell<T> {}

impl<T: ?Sized + Debug> Debug for AtomicRefCell<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { self.borrow().fmt(f) }
}

impl<'a, T: ?Sized + Debug> Debug for AtomicRef<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { self.deref().fmt(f) }
}

impl<'a, T: ?Sized + Debug> Debug for AtomicRefMut<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { self.deref().fmt(f) }
}

impl<'a, T> !Send for AtomicRefMut<'a, T> {}
