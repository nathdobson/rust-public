#![allow(dead_code)]
#![allow(unused_imports)]
#![feature(test)]
#![feature(bench_black_box)]
#![feature(cell_update)]
#![feature(unchecked_math)]
#![feature(negative_impls)]

extern crate test;

use std::cell::{Cell, RefCell, UnsafeCell};
use std::hint::black_box;
use std::mem;
use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

use fast_thread_id::FastThreadId;
use test::Bencher;

pub struct ReentrantRefCell<T> {
    owner: AtomicUsize,
    value: UnsafeCell<T>,
}

struct LockGuard<'a>(&'a AtomicUsize);

impl<T> ReentrantRefCell<T> {
    pub fn new(value: T) -> Self {
        ReentrantRefCell {
            owner: AtomicUsize::new(0),
            value: UnsafeCell::new(value),
        }
    }
    pub fn new_pinned(value: T) -> Self {
        ReentrantRefCell {
            owner: AtomicUsize::new(FastThreadId::get().into_usize()),
            value: UnsafeCell::new(value),
        }
    }
    #[inline]
    fn lock<'a>(&'a self, tid: FastThreadId) -> Result<LockGuard<'a>, usize> {
        self.owner
            .compare_exchange(0, tid.into_usize(), Acquire, Relaxed)?;
        Ok(LockGuard(&self.owner))
    }
    pub fn pin_outer(&self) { mem::forget(self.lock(FastThreadId::get()).expect("Already locked")) }
    #[inline(never)]
    pub fn with_outer<F, O>(&self, f: F) -> O
    where
        F: for<'a> FnOnce(&'a T) -> O,
    {
        let tid = FastThreadId::get();
        let _guard = self.lock(tid).unwrap_or_else(|existing| {
            if existing == tid.into_usize() {
                panic!("Already locked by current thread")
            } else {
                panic!("Already locked by other thread")
            }
        });
        unsafe { f(&*self.value.get()) }
    }
    #[inline]
    pub fn with_inner<F, O>(&self, f: F) -> O
    where
        F: for<'a> FnOnce(&'a T) -> O,
    {
        let tid = FastThreadId::get().into_usize();
        let owner = self.owner.load(Relaxed);
        if owner != tid {
            #[inline(never)]
            #[cold]
            fn panic_for_owner(owner: usize) {
                if owner == 0 {
                    panic!("Not already locked")
                } else {
                    panic!("Already locked by other thread")
                }
            }
            panic_for_owner(owner)
        }
        unsafe { f(&*self.value.get()) }
    }
    #[inline]
    pub fn with_reentrant<F, O>(&self, f: F) -> O
    where
        F: for<'a> FnOnce(&'a T) -> O,
    {
        let tid = FastThreadId::get();
        let old = self.owner.load(Relaxed);
        let _guard;
        if old != tid.into_usize() {
            _guard = self
                .lock(tid)
                .unwrap_or_else(|_| panic!("Already locked by other thread"));
        }
        unsafe { f(&*self.value.get()) }
    }
}

impl<'a> Drop for LockGuard<'a> {
    #[inline]
    fn drop(&mut self) { self.0.store(0, Release); }
}

unsafe impl<T: Send> Send for ReentrantRefCell<T> {}

unsafe impl<T: Send> Sync for ReentrantRefCell<T> {}

impl<'a> !Send for LockGuard<'a> {}

#[test]
fn test_simple() {
    let cell = ReentrantRefCell::new(1);
    assert_eq!(2, cell.with_outer(|_| cell.with_inner(|_| 2)));
}

#[test]
fn test_pinned() {
    let cell = ReentrantRefCell::new_pinned(1);
    assert_eq!(2, cell.with_inner(|_| 2));
}
