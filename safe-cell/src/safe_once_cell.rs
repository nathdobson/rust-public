use std::cell::{Cell, UnsafeCell};
use std::mem;
use std::mem::MaybeUninit;
use std::sync::atomic::Ordering::Release;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread::panicking;

use ondrop::OnDrop;
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};

const UNINIT: u8 = 0;
const INIT: u8 = 1;
const POISON: u8 = 2;

pub struct SafeOnceCell<T = ()> {
    initializing: ReentrantMutex<Cell<bool>>,
    initialized: AtomicU8,
    value: UnsafeCell<MaybeUninit<T>>,
}

pub struct SafeOnceGuard<'a, T> {
    once: Option<&'a SafeOnceCell<T>>,
    _inner: ReentrantMutexGuard<'a, Cell<bool>>,
}

impl<T> SafeOnceCell<T> {
    pub const fn new() -> Self {
        SafeOnceCell {
            initializing: ReentrantMutex::new(Cell::new(false)),
            initialized: AtomicU8::new(UNINIT),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
    pub fn get(&self) -> Option<&T> {
        unsafe {
            if self.initialized.load(Ordering::Acquire) == INIT {
                Some((*self.value.get()).assume_init_ref())
            } else {
                None
            }
        }
    }
    pub fn try_lock<'a>(&'a self) -> Result<&'a T, SafeOnceGuard<'a, T>> {
        unsafe {
            if self.initialized.load(Ordering::Acquire) == INIT {
                return Ok((*self.value.get()).assume_init_ref());
            }
            let lock = self.initializing.lock();
            if self.initialized.load(Ordering::Acquire) == INIT {
                return Ok((*self.value.get()).assume_init_ref());
            }
            if lock.replace(true) {
                panic!("Deadlock in initialization.")
            }
            Err(SafeOnceGuard {
                once: Some(self),
                _inner: lock,
            })
        }
    }
    pub fn get_or_init<F: FnOnce() -> T>(&self, init: F) -> &T {
        match self.try_lock() {
            Ok(x) => x,
            Err(guard) => guard.init(init()),
        }
    }
    pub fn into_inner(mut self) -> Option<T> {
        unsafe {
            if mem::replace(self.initialized.get_mut(), POISON) == INIT {
                Some((*self.value.get()).assume_init_read())
            } else {
                None
            }
        }
    }
}

impl<'a, T> SafeOnceGuard<'a, T> {
    pub fn init(mut self, x: T) -> &'a T {
        unsafe {
            let once = self.once.take().unwrap();
            (*once.value.get()).write(x);
            once.initialized.store(INIT, Release);
            (*once.value.get()).assume_init_ref()
        }
    }
}

impl<'a, T> Drop for SafeOnceGuard<'a, T> {
    fn drop(&mut self) {
        if let Some(once) = self.once {
            if panicking() {
                once.initialized.store(POISON, Release);
            }
        }
    }
}

impl<T> From<T> for SafeOnceCell<T> {
    fn from(x: T) -> Self {
        SafeOnceCell {
            initializing: ReentrantMutex::new(Cell::new(true)),
            initialized: AtomicU8::new(INIT),
            value: UnsafeCell::new(MaybeUninit::new(x)),
        }
    }
}

unsafe impl<T: Send> Send for SafeOnceCell<T> {}

unsafe impl<T: Sync + Send> Sync for SafeOnceCell<T> {}

impl<T: Clone> Clone for SafeOnceCell<T> {
    fn clone(&self) -> Self {
        if let Some(x) = self.get() {
            SafeOnceCell::from(x.clone())
        } else {
            SafeOnceCell::new()
        }
    }
}

impl<T> Default for SafeOnceCell<T> {
    fn default() -> Self { SafeOnceCell::new() }
}
