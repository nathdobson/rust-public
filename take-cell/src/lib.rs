#![feature(allocator_api)]

use std::alloc::{AllocError, Allocator, Layout};
use std::cell::UnsafeCell;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

pub struct TakeCell<T: ?Sized> {
    taken: AtomicBool,
    value: UnsafeCell<ManuallyDrop<T>>,
}

impl<T> TakeCell<T> {
    pub fn new(value: T) -> Self
    where
        T: Sized,
    {
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

struct NoopAllocator;

unsafe impl Allocator for NoopAllocator {
    fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> { Err(AllocError) }
    unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {}
}

impl<T: FnOnce() + ?Sized> TakeCell<T> {
    pub fn call(&self) {
        unsafe {
            if !self.taken.swap(true, Relaxed) {
                let x = &mut **self.value.get();
                Box::from_raw_in(x, NoopAllocator)()
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
