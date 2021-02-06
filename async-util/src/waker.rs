use std::task::{Waker, RawWakerVTable, RawWaker};

#[cfg(loom)]
pub(crate) use loom::sync::atomic::AtomicUsize;
#[cfg(not(loom))]
pub(crate) use std::sync::atomic::AtomicUsize;


#[cfg(loom)]
pub(crate) use loom::sync::atomic::AtomicPtr;
#[cfg(not(loom))]
pub(crate) use std::sync::atomic::AtomicPtr;

use std::sync::atomic::Ordering::*;
use std::mem;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;

pub struct AtomicWaker {
    state: AtomicUsize,
    data: AtomicPtr<()>,
    vtable: AtomicPtr<RawWakerVTable>,
}

// trait Value {
//     type V;
//     fn value(self) -> V;
// }
//
// impl<T> Value for Result<T, T> {
//     type V = T;
//
//     fn value(self) -> T {
//         match self {
//             Ok(x) => x,
//             Err(x) => x,
//         }
//     }
// }

unsafe fn from_waker(waker: Waker) -> (*mut (), *mut RawWakerVTable) {
    mem::transmute(waker)
}

unsafe fn to_waker(waker: (*mut (), *mut RawWakerVTable)) -> Waker {
    mem::transmute(waker)
}

const EMPTY: usize = 0;
const SLEEPING: usize = 1;
const REGISTERING: usize = 2;
const WAKING: usize = 3;

trait AtomicExt {
    fn compare_transact_weak(&self, current: &mut usize, new: usize, success: Ordering, failure: Ordering) -> bool;
}

impl AtomicExt for AtomicUsize {
    fn compare_transact_weak(&self, current: &mut usize, new: usize, success: Ordering, failure: Ordering) -> bool {
        match self.compare_exchange_weak(*current, new, success, failure) {
            Ok(_) => {
                *current = new;
                true
            }
            Err(actual) => {
                *current = actual;
                false
            }
        }
    }
}

impl AtomicWaker {
    pub fn new() -> Self {
        AtomicWaker {
            state: AtomicUsize::new(0),
            data: AtomicPtr::new(null_mut()),
            vtable: AtomicPtr::new(null_mut()),
        }
    }
    #[cfg(test)]
    unsafe fn state(&self) -> Option<RawWaker> {
        match self.state.load(Relaxed) {
            EMPTY => None,
            SLEEPING => {
                Some(mem::transmute::<(*mut (), *mut RawWakerVTable), RawWaker>(
                    (self.data.load(Relaxed), self.vtable.load(Relaxed))))
            }
            _ => panic!(),
        }
    }
    pub unsafe fn register(&self, waker: &Waker) {
        let mut old_state = self.state.load(Relaxed);
        loop {
            match old_state {
                EMPTY => {
                    if self.state.compare_transact_weak(&mut old_state, REGISTERING, Acquire, Relaxed) {
                        break;
                    } else { continue; }
                }
                SLEEPING => {
                    if self.state.compare_transact_weak(&mut old_state, REGISTERING, Acquire, Relaxed) {
                        let old_vtable = self.vtable.load(Relaxed);
                        let old_data = self.data.load(Relaxed);
                        mem::drop(to_waker((old_data, old_vtable)));
                        break;
                    } else { continue; }
                }
                REGISTERING => panic!(),
                WAKING => {
                    if self.state.compare_transact_weak(&mut old_state, REGISTERING, Acquire, Relaxed) {
                        let old_vtable = self.vtable.load(Relaxed);
                        let old_data = self.data.load(Relaxed);
                        to_waker((old_data, old_vtable)).wake();
                        break;
                    } else { continue; }
                }
                _ => unreachable!(),
            }
        }
        let (new_data, new_vtable) = from_waker(waker.clone());
        self.vtable.store(new_vtable, Relaxed);
        self.data.store(new_data, Relaxed);
        loop {
            match old_state {
                EMPTY => {
                    to_waker((new_data, new_vtable)).wake();
                    break;
                }
                SLEEPING => panic!(),
                WAKING => panic!(),
                REGISTERING => {
                    if self.state.compare_transact_weak(&mut old_state, SLEEPING, Release, Relaxed) {
                        break;
                    } else { continue; }
                }
                _ => panic!(),
            }
        }
    }
    pub unsafe fn wake(&self) {
        let mut old_state = self.state.load(Relaxed);
        loop {
            match old_state {
                EMPTY => return,
                SLEEPING => {
                    if self.state.compare_transact_weak(&mut old_state, WAKING, Acquire, Relaxed) {
                        break;
                    } else { continue; }
                }
                REGISTERING => {
                    if self.state.compare_transact_weak(&mut old_state, EMPTY, Relaxed, Relaxed) {
                        return;
                    } else { continue; }
                }
                WAKING => panic!(),
                _ => panic!(),
            }
        }
        let old_vtable = self.vtable.load(Relaxed);
        let old_data = self.data.load(Relaxed);
        loop {
            match old_state {
                EMPTY => return,
                SLEEPING => return,
                REGISTERING => {
                    if self.state.compare_transact_weak(&mut old_state, EMPTY, Relaxed, Relaxed) {
                        return;
                    } else { continue; }
                }
                WAKING => {
                    if self.state.compare_transact_weak(&mut old_state, EMPTY, Release, Relaxed) {
                        break;
                    } else { continue; }
                }
                _ => panic!(),
            }
        }
        to_waker((old_data, old_vtable)).wake();
    }
}

#[cfg(test)]
pub mod test {
    use std::task::{RawWakerVTable, Waker, RawWaker};
    use std::sync::atomic::Ordering::Relaxed;
    use crate::waker::AtomicWaker;
    use std::mem;

    #[cfg(loom)]
    pub(crate) use loom::sync::atomic::AtomicUsize;
    #[cfg(not(loom))]
    pub(crate) use std::sync::atomic::AtomicUsize;

    pub struct TestWaker {
        refs: AtomicUsize,
        wakes: AtomicUsize,
        vtable: &'static RawWakerVTable,
    }

    pub const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |x| unsafe { (x as *const TestWaker).waker_clone() },
        |x| unsafe { (x as *const TestWaker).waker_wake() },
        |x| unsafe { (x as *const TestWaker).waker_wake_by_ref() },
        |x| unsafe { (x as *const TestWaker).waker_drop() },
    );

    impl TestWaker {
        pub fn new() -> (&'static TestWaker, Waker) {
            Self::from_vtable(&VTABLE)
        }
        fn from_vtable(vtable: &'static RawWakerVTable) -> (&'static TestWaker, Waker) {
            let inner = Box::leak(Box::new(Self {
                refs: AtomicUsize::new(1),
                wakes: AtomicUsize::new(0),
                vtable,
            }));
            let waker = unsafe { Waker::from_raw(inner.raw()) };
            (inner, waker)
        }

        fn raw(&self) -> RawWaker {
            RawWaker::new(self as *const Self as *const (), self.vtable)
        }

        unsafe fn waker_clone(self: *const Self) -> RawWaker {
            assert!((*self).refs.fetch_add(1, Relaxed) > 0);
            RawWaker::new(self as *const (), (*self).vtable)
        }

        unsafe fn waker_wake(self: *const Self) {
            self.waker_wake_by_ref();
            self.waker_drop();
        }

        unsafe fn waker_wake_by_ref(self: *const Self) {
            assert!((*self).refs.load(Relaxed) > 0);
            (*self).wakes.fetch_add(1, Relaxed);
        }

        unsafe fn waker_drop(self: *const Self) {
            assert!((*self).refs.fetch_sub(1, Relaxed) > 0);
        }

        fn load(&self) -> (usize, usize) {
            (self.refs.load(Relaxed), self.wakes.load(Relaxed))
        }
    }


    #[test]
    fn test() {
        unsafe {
            let table1: &'static mut RawWakerVTable = Box::leak(Box::new(VTABLE.clone()));
            let table2: &'static mut RawWakerVTable = Box::leak(Box::new(VTABLE.clone()));
            let (inner1, waker1) = TestWaker::from_vtable(table1);
            let (inner2, waker2) = TestWaker::from_vtable(table2);

            let atomic = AtomicWaker::new();
            assert_eq!(atomic.state(), None);
            assert_eq!(inner1.load(), (1, 0));
            assert_eq!(inner2.load(), (1, 0));

            atomic.register(&waker1);
            assert_eq!(atomic.state(), Some(inner1.raw()));
            assert_eq!(inner1.load(), (2, 0));
            assert_eq!(inner2.load(), (1, 0));

            atomic.register(&waker2);
            assert_eq!(atomic.state(), Some(inner2.raw()));
            assert_eq!(inner1.load(), (1, 0));
            assert_eq!(inner2.load(), (2, 0));

            atomic.wake();
            assert!(atomic.state().is_none());
            assert_eq!(inner1.load(), (1, 0));
            assert_eq!(inner2.load(), (1, 1));
        }
    }

    #[test]
    #[cfg(loom)]
    fn test_loom() {
        use loom::sync::Arc;
        use loom::sync::atomic::AtomicUsize;
        use loom::sync::atomic::Ordering::{Acquire, Release, Relaxed};
        use loom::thread;
        loom::model(|| {
            unsafe {
                let table1: &'static mut RawWakerVTable = Box::leak(Box::new(VTABLE.clone()));
                let table2: &'static mut RawWakerVTable = Box::leak(Box::new(VTABLE.clone()));
                let (inner1, waker1) = TestWaker::new(table1);
                let (inner2, waker2) = TestWaker::new(table2);

                let atomic_waker = Arc::new(AtomicWaker::new());
                let handle = thread::spawn({
                    let atomic_waker = atomic_waker.clone();
                    move || {
                        atomic_waker.wake();
                        atomic_waker.wake();
                    }
                });
                atomic_waker.register(&waker1);
                mem::drop(waker1);
                atomic_waker.register(&waker2);
                mem::drop(waker2);
                handle.join().unwrap();
                if let Some(stored) = atomic_waker.state() {
                    assert_eq!(stored, inner2.raw());
                    match (inner1.load(), inner2.load()) {
                        ((0, 0 | 1), (1, 0)) => {}
                        unexpected => panic!("{:?}", unexpected),
                    }
                } else {
                    match (inner1.load(), inner2.load()) {
                        ((0, 0 | 1), (0, 1)) => {}
                        unexpected => panic!("{:?}", unexpected),
                    }
                }
            }
        });
    }
}