use crate::atomic_refcell::AtomicRefCell;
use crate::weak_vec::WeakVec;
use std::sync::atomic::AtomicBool;
use std::mem::{MaybeUninit, ManuallyDrop};
use std::cell::UnsafeCell;
use std::sync::atomic::Ordering::Relaxed;
use std::alloc::{Allocator, Layout, AllocError};
use std::ptr::NonNull;
use crate::fun::call_once_raw;

pub struct TakeCell<T: ?Sized> {
    taken: AtomicBool,
    value: UnsafeCell<ManuallyDrop<T>>,
}

impl<T> TakeCell<T> {
    pub fn new(value: T) -> Self where T: Sized {
        TakeCell {
            taken: AtomicBool::new(false),
            value: UnsafeCell::new(ManuallyDrop::new(value)),
        }
    }
    pub fn take(&self) -> Option<T> {
        unsafe {
            if !self.taken.swap(true, Relaxed) {
                Some(ManuallyDrop::take(&mut *self.value.get()))
            } else {
                None
            }
        }
    }
}

impl<T: FnOnce() + ?Sized> TakeCell<T> {
    pub fn call(&self) {
        unsafe {
            if !self.taken.swap(true, Relaxed) {
                let x = &mut **self.value.get();
                call_once_raw(x as *mut T);
            }
        }
    }
}

impl<T: ?Sized> Drop for TakeCell<T> {
    fn drop(&mut self) {
        unsafe {
            if !*self.taken.get_mut() {
                ManuallyDrop::drop(self.value.get_mut())
            }
        }
    }
}

unsafe impl<T: ?Sized> Send for TakeCell<T> {}

unsafe impl<T: ?Sized> Sync for TakeCell<T> {}

