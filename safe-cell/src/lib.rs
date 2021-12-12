#![allow(unused_imports)]

use std::cell::{Cell, UnsafeCell};
use std::ops::Deref;
use std::panic::resume_unwind;
use std::sync::{Arc, Barrier};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::Ordering::Release;
use std::thread;
use std::thread::ThreadId;
use std::time::Duration;
use ondrop::OnDrop;
use parking_lot::ReentrantMutex;

pub struct SafeOnceCell<T> {
    initializing: ReentrantMutex<Cell<bool>>,
    initialized: AtomicBool,
    value: UnsafeCell<Option<T>>,
}

pub struct SafeLazy<T, F = fn() -> T> {
    cell: SafeOnceCell<T>,
    init: Cell<Option<F>>,
}

impl<T> SafeOnceCell<T> {
    pub const fn new() -> Self {
        SafeOnceCell {
            initializing: ReentrantMutex::new(Cell::new(false)),
            initialized: AtomicBool::new(false),
            value: UnsafeCell::new(None),
        }
    }
    pub fn get_or_init<F: FnOnce() -> T>(&self, init: F) -> &T {
        self.get_or_init_impl(init).expect("Poisoned")
    }
    fn get_or_init_impl<F: FnOnce() -> T>(&self, init: F) -> Option<&T> {
        unsafe {
            if self.initialized.load(Ordering::Acquire) {
                return (*self.value.get()).as_ref();
            }
            let lock = self.initializing.lock();
            if self.initialized.load(Ordering::Acquire) {
                return (*self.value.get()).as_ref();
            }
            let _done = OnDrop::new(|| self.initialized.store(true, Release));
            if lock.replace(true) { panic!("Deadlock in initialization.") }
            let value = &mut *self.value.get();
            *value = Some(init());
            (*self.value.get()).as_ref()
        }
    }
}

impl<T, F> SafeLazy<T, F> {
    pub const fn new(f: F) -> Self {
        SafeLazy { cell: SafeOnceCell::new(), init: Cell::new(Some(f)) }
    }
}

impl<T, F: FnOnce() -> T> Deref for SafeLazy<T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.cell.get_or_init(|| {
            self.init.take().unwrap()()
        })
    }
}

unsafe impl<T: Send> Send for SafeOnceCell<T> {}

unsafe impl<T: Sync + Send> Sync for SafeOnceCell<T> {}

unsafe impl<T: Send, F: Send> Send for SafeLazy<T, F> {}

unsafe impl<T: Sync + Send, F: Send> Sync for SafeLazy<T, F> {}


#[test]
fn test_simple() {
    let foo = SafeOnceCell::new();
    assert_eq!(&42u8, foo.get_or_init(|| 42u8));
}

#[cfg(test)]
fn parallel(threads: usize, f: impl 'static + Send + Sync + Fn()) {
    let barrier = Arc::new(Barrier::new(threads));
    let f = Arc::new(f);
    (0..threads).map(|_| thread::spawn({
        let barrier = barrier.clone();
        let f = f.clone();
        move || {
            barrier.wait();
            f();
        }
    })).collect::<Vec<_>>().into_iter().for_each(|x| { x.join().unwrap_or_else(|x| resume_unwind(x)); });
}

#[test]
fn test_racy() {
    let cell = Arc::new(SafeOnceCell::new());
    parallel(1000, {
        let cell = cell.clone();
        move || {
            assert_eq!(cell.get_or_init(|| 42u8), &42u8);
        }
    });
    assert_eq!(&42, cell.get_or_init(|| panic!()));
}

#[test]
#[should_panic(expected = "Deadlock")]
fn test_reentrant() {
    let cell = SafeOnceCell::new();
    cell.get_or_init(|| *cell.get_or_init(|| 42));
}

#[test]
#[should_panic(expected = "Poisoned")]
fn test_racy_reentrant() {
    use rand::thread_rng;
    use rand::Rng;


    let cell = Arc::new(SafeOnceCell::new());
    parallel(100, {
        let cell = cell.clone();
        move || {
            cell.get_or_init(|| {
                thread::sleep(Duration::from_millis(thread_rng().gen_range(0..100)));
                *cell.get_or_init(|| 42)
            });
        }
    });
}