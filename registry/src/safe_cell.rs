use std::cell::{Cell, UnsafeCell};
use std::lazy::SyncOnceCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use parking_lot::Mutex;
use std::{mem, thread};
use std::ops::Deref;
use std::panic::{catch_unwind, resume_unwind};
use std::sync::atomic::Ordering::Release;
use std::sync::{Arc, Barrier};
use std::thread::{current, Thread, ThreadId};
use std::time::Duration;
use parking_lot::Condvar;
use rand::{Rng, thread_rng};

#[derive(Copy, Clone)]
enum State {
    Uninit,
    Initing(ThreadId),
    Init,
    Poisoned,
}

pub struct SafeOnceCell<T> {
    value: SyncOnceCell<T>,
    mutex: Mutex<State>,
    condvar: Condvar,
}

pub struct SafeLazy<T, F = fn() -> T> {
    cell: SafeOnceCell<T>,
    init: Cell<Option<F>>,
}

impl<T> SafeOnceCell<T> {
    pub const fn new() -> Self {
        SafeOnceCell {
            value: SyncOnceCell::new(),
            mutex: Mutex::new(State::Uninit),
            condvar: Condvar::new(),
        }
    }
    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        if let Some(x) = self.value.get() {
            return x;
        }
        let mut lock = self.mutex.lock();
        loop {
            match *lock {
                State::Uninit => {
                    *lock = State::Initing(thread::current().id());
                    mem::drop(lock);
                    struct OnDrop<'a, T>(Option<&'a SafeOnceCell<T>>);
                    impl<'a, T> Drop for OnDrop<'a, T> {
                        fn drop(&mut self) {
                            if let Some(x) = self.0.take() {
                                *x.mutex.lock() = State::Poisoned;
                                x.condvar.notify_all();
                            }
                        }
                    }
                    let mut on_drop = OnDrop(Some(self));
                    let value = f();
                    on_drop.0.take();
                    self.value.set(value).ok().unwrap();
                    *self.mutex.lock() = State::Init;
                    self.condvar.notify_all();
                    break;
                }
                State::Initing(id) => {
                    if id == thread::current().id() {
                        panic!("Deadlock");
                    } else {
                        self.condvar.wait(&mut lock);
                    }
                }
                State::Init => {
                    break;
                }
                State::Poisoned => {
                    panic!("Poisoned");
                }
            }
        }
        return self.value.get().unwrap();
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

unsafe impl<T: Send, F: Send> Send for SafeLazy<T, F> {}

unsafe impl<T: Sync + Send, F: Send> Sync for SafeLazy<T, F> {}

#[test]
fn test_simple() {
    let foo = SafeOnceCell::new();
    assert_eq!(&42u8, foo.get_or_init(|| 42u8));
}

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