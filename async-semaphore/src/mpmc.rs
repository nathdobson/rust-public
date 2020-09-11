use futures::pending;
use futures::task::{AtomicWaker, Context, Poll};
use std::sync::atomic::{AtomicU64, Ordering, AtomicU128, AtomicUsize};
use std::{mem, fmt};
use crate::{future_waker, LockError, future_wait, AssertSyncSend, Underflow};
use std::marker::PhantomData;
use std::ptr::null;
use std::mem::MaybeUninit;
use std::cell::UnsafeCell;
use std::task::Waker;
use std::sync::Mutex;
use crate::atomic::{Atomic, Encoder};
use std::future::Future;
use std::pin::Pin;
use std::ops::DerefMut;
use std::fmt::{Debug, Formatter};
//use crate::mpmc::PtrSeq::{Ptr, Seq};


#[repr(align(64))]
pub struct Waiter {
    waker: Atomic<Option<Waker>, AtomicU128, WakerEncoder>,
    amount: UnsafeCell<u64>,
    next: Atomic<*const Waiter, AtomicU64>,
}

struct WakerEncoder;

impl Encoder<Option<Waker>, u128> for WakerEncoder {
    unsafe fn decode(x: u128) -> Option<Waker> {
        match x {
            0 => None,
            _ => Some(mem::transmute(x)),
        }
    }
    unsafe fn encode(x: Option<Waker>) -> u128 {
        match x {
            None => 0,
            Some(x) => mem::transmute(x),
        }
    }
}

// #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
// enum PtrSeq {
//     Ptr(*const Waiter),
//     Seq(u64),
// }
//
// impl PtrSeq {
//     fn ptr(self) -> Option<*const Waiter> {
//         match self {
//             PtrSeq::Ptr(ptr) => Some(ptr),
//             PtrSeq::Seq(_) => None,
//         }
//     }
// }
//
// struct PtrSeqEncoder;
//
// impl Encoder<PtrSeq, u64> for PtrSeqEncoder {
//     unsafe fn encode(x: PtrSeq) -> u64 {
//         unimplemented!()
//     }
//
//     unsafe fn decode(x: u64) -> PtrSeq {
//         unimplemented!()
//     }
// }


pub struct Semaphore {
    capacity: u64,
    state: Atomic<State, AtomicU128, StateEncoder>,
    front: UnsafeCell<*const Waiter>,
    free: Atomic<(*const Waiter, u64), AtomicU128>,
    //allocs: AtomicU64,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct State {
    available: u64,
    back: *const Waiter,
    locked: bool,
    queued: bool,
}

pub struct SemaphoreGuard<'a> {
    semaphore: &'a Semaphore,
    amount: u64,
}

unsafe impl Sync for Semaphore {}

unsafe impl Send for Semaphore {}

struct StateEncoder;

impl Encoder<State, u128> for StateEncoder {
    unsafe fn encode(state: State) -> u128 {
        ((state.available as u128) << 64) |
            (state.back as u128) |
            (if state.locked { 2 } else { 0 }) |
            (if state.queued { 1 } else { 0 })
    }

    unsafe fn decode(x: u128) -> State {
        State {
            available: (x >> 64) as u64,
            back: ((x as u64) & (!3u64)) as *const Waiter,
            locked: (x & 2) == 2,
            queued: (x & 1) == 1,
        }
    }
}

impl SemaphoreGuard<'_> {
    pub fn forget(self) {
        mem::forget(self)
    }
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        self.semaphore.release(self.amount).unwrap();
    }
}

struct DebugPtr(*const Waiter);

impl Debug for DebugPtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            write!(f, "{:?}", self.0)?;
            if self.0 != null() {
                write!(f, " {:?} {:?}", *(*self.0).amount.get(), DebugPtr((*self.0).next.load(Ordering::SeqCst)))?;
            }
            Ok(())
        }
    }
}

impl Debug for Semaphore {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            let mut w = f.debug_struct("Semaphore");
            w.field("C", &self.capacity);
            let state = self.state.load(Ordering::SeqCst);
            w.field("A", &state.available);
            w.field("Q", &state.queued);
            w.field("L", &state.locked);
            w.field("B", &DebugPtr(state.back));
            w.field("F", &DebugPtr(*self.front.get()));
            let (free, free_gen) = self.free.load(Ordering::SeqCst);
            w.field("P", &(DebugPtr(free), free_gen));
            //w.field("N", &(self.allocs.load(Ordering::SeqCst)));
            w.finish()
        }
    }
}

impl Semaphore {
    pub fn new(capacity: u64) -> Semaphore {
        Semaphore {
            capacity,
            state: Atomic::new(State {
                available: capacity,
                back: null(),
                locked: false,
                queued: false,
            }),
            front: UnsafeCell::new(null()),
            free: Atomic::new((null(), 0)),
            //allocs: AtomicU64::new(0),
        }
    }
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
    pub fn available(&self) -> u64 {
        self.state.load(Ordering::SeqCst).available
    }
    pub async fn try_acquire(&self, amount: u64) -> Result<SemaphoreGuard<'_>, LockError> {
        if amount > self.capacity {
            return Err(LockError::WouldDeadlock);
        }
        if self.state.fetch_update(
            Ordering::AcqRel, Ordering::Acquire,
            |state| {
                if amount <= state.available && !state.queued {
                    Some(State { available: state.available - amount, ..state })
                } else {
                    None
                }
            }).is_ok() {
            Ok(SemaphoreGuard { semaphore: self, amount })
        } else {
            Err(LockError::WouldBlock)
        }
    }
    unsafe fn allocate(&self) -> *const Waiter {
        //self.allocs.fetch_add(1, Ordering::SeqCst);
        if let Ok((free, _free_ver)) = self.free.fetch_update(
            Ordering::AcqRel, Ordering::Acquire,
            |(free, free_ver)| {
                if free == null() {
                    None
                } else {
                    Some(((*free).next
                              .load(Ordering::Acquire), free_ver + 1))
                }
            }) {
            free
        } else {
            Box::into_raw(Box::new(Waiter {
                waker: Atomic::new(None),
                amount: UnsafeCell::new(0),
                next: Atomic::new(null()),
            }))
        }
    }
    unsafe fn free(&self, node: *const Waiter) {
        (*node).waker.swap(None, Ordering::Relaxed);
        self.free.fetch_update(
            Ordering::AcqRel, Ordering::Acquire,
            |(free, free_ver)| {
                (*node).next.store(free, Ordering::Relaxed);
                Some((node, free_ver + 1))
            }).unwrap();
    }

    pub fn acquire(&self, amount: u64) -> impl Future<Output=Result<SemaphoreGuard<'_>, LockError>> {
        unsafe { AssertSyncSend::new(self.acquire_impl(amount)) }
    }

    async fn acquire_impl(&self, amount: u64) -> Result<SemaphoreGuard<'_>, LockError> {
        unsafe {
            match self.try_acquire(amount).await {
                Ok(guard) => return Ok(guard),
                Err(LockError::WouldDeadlock) => return Err(LockError::WouldDeadlock),
                Err(LockError::WouldBlock) => {}
            }
            let node = self.allocate();
            (*(*node).amount.get()) = amount;
            mem::drop((*node).waker.swap(Some(future_waker().await), Ordering::Relaxed));
            let mut acquired = false;
            self.state.fetch_update(
                Ordering::AcqRel, Ordering::Acquire,
                |state| {
                    if amount <= state.available && !state.queued {
                        acquired = true;
                        Some(State { available: state.available - amount, ..state })
                    } else {
                        acquired = false;
                        (*node).next.store(state.back, Ordering::Relaxed);
                        Some(State { queued: true, back: node, ..state })
                    }
                }).unwrap();
            if acquired {
                self.free(node);
                return Ok(SemaphoreGuard { semaphore: self, amount });
            }
            loop {
                future_wait(|| {
                    let old = (*node).waker.swap(None, Ordering::AcqRel);
                    if old.is_none() {
                        assert_eq!(*self.front.get(), node);
                        *self.front.get() = (*node).next.load(Ordering::Relaxed);
                        self.free(node);
                        if self.state.fetch_update(
                            Ordering::AcqRel, Ordering::Acquire,
                            |state| {
                                if *self.front.get() == null() && state.back == null() {
                                    //println!("unlocking in cancellation");
                                    Some(State { queued: false, locked: false, ..state })
                                } else {
                                    None
                                }
                            }).is_ok() {
                            return;
                        };
                        self.unlock();
                    }
                }).await;
                let old = (*node).waker.swap(Some(future_waker().await), Ordering::AcqRel);
                if old.is_some() {
                    continue;
                }
                let next = (*node).next.load(Ordering::Relaxed);
                assert_eq!(*self.front.get(), node);
                let mut acquired = false;
                self.state.fetch_update(
                    Ordering::AcqRel, Ordering::Acquire,
                    |state| {
                        if amount <= state.available {
                            *self.front.get() = next;
                            acquired = true;
                            //println!("unlocking after acquire");
                            Some(State {
                                available: state.available - amount,
                                back: state.back,
                                locked: true,
                                queued: next != null() || state.back != null(),
                            })
                        } else {
                            *self.front.get() = node;
                            acquired = false;
                            //println!("unlocking because of available");
                            Some(State { locked: false, ..state })
                        }
                    },
                ).unwrap();
                if acquired {
                    self.unlock();
                    self.free(node);

                    return Ok(SemaphoreGuard { semaphore: self, amount });
                }
            }
        }
    }
    unsafe fn unlock(&self) {
        loop {
            let state = self.state.load(Ordering::Acquire);
            if !state.queued {
                assert_eq!(*self.front.get(), null());
                if self.state.compare_exchange_weak(state, State {
                    locked: false,
                    ..state
                }, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                    return;
                } else {
                    continue;
                }
            }
            if *self.front.get() == null() {
                let mut back = self.state.fetch_update(
                    Ordering::AcqRel, Ordering::Acquire,
                    |state| {
                        Some(State { back: null(), ..state })
                    }).unwrap().back;
                while back != null() {
                    let next = (*back).next.load(Ordering::Relaxed);
                    (*back).next.store(*self.front.get(), Ordering::Relaxed);
                    *self.front.get() = back;
                    back = next;
                }
                assert_ne!((*self.front.get()), null());
                continue;
            }
            if let Some(waker) =
            (**self.front.get()).waker.swap(None, Ordering::AcqRel) {
                waker.wake();
                return;
            } else {
                let front = *self.front.get();
                *self.front.get() = (*front).next.load(Ordering::Relaxed);
                self.free(front);
                if self.state.fetch_update(
                    Ordering::AcqRel, Ordering::Acquire,
                    |state| {
                        if *self.front.get() == null() && state.back == null() {
                            Some(State { queued: false, locked: false, ..state })
                        } else {
                            None
                        }
                    }).is_ok() {
                    return;
                };
            }
        }
    }

    pub fn release(&self, amount: u64) -> Result<(), Underflow> {
        unsafe {
            match self.state.fetch_update(
                Ordering::AcqRel, Ordering::Acquire,
                |state| {
                    if state.available + amount > self.capacity {
                        None
                    } else {
                        Some(State {
                            available: state.available + amount,
                            locked: true,
                            ..state
                        })
                    }
                }) {
                Ok(State { locked: true, .. }) => return Ok(()),
                Err(_) => return Err(Underflow),
                _ => {}
            }
            self.unlock();
            Ok(())
        }
    }
}

impl Drop for Waiter {
    fn drop(&mut self) {
        self.waker.swap(None, Ordering::Relaxed);
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            //println!("Dropping {:?}", self);
            for &(mut ptr) in &[*self.front.get(),
                self.free.load(Ordering::Relaxed).0,
                self.state.load(Ordering::Relaxed).back] {
                while ptr != null() {
                    let next = (*ptr).next.load(Ordering::Relaxed);
                    Box::from_raw(ptr as *mut Waiter);
                    ptr = next;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mpmc::{Semaphore, SemaphoreGuard};
    use futures::executor::{LocalPool, ThreadPool, block_on};
    use futures::task::{LocalSpawnExt, SpawnExt};
    use rand::{thread_rng, Rng, SeedableRng};
    use std::sync::{Arc, Mutex};
    use std::rc::Rc;
    use std::cell::{Cell, RefCell};
    use rand_xorshift::XorShiftRng;
    use std::sync::atomic::{AtomicUsize, Ordering, AtomicU64};
    use std::mem;
    use futures::StreamExt;
    use itertools::Itertools;
    use std::time::Duration;
    use async_std::task::sleep;
    use crate::{LockError, Underflow};

    #[test]
    fn test_simple() {
        let semaphore = Rc::new(Semaphore::new(10));
        let mut pool = LocalPool::new();
        let spawner = pool.spawner();
        spawner.spawn_local({
            let semaphore = semaphore.clone();
            async move {
                semaphore.acquire(10).await.unwrap().forget();
                semaphore.acquire(10).await.unwrap().forget();
            }
        }).unwrap();
        pool.run_until_stalled();
        semaphore.release(10).unwrap();
        pool.run();
    }

    // lazy_static! {
    //     static TIDS: AtomicU64 = AtomicU64::new();
    // }
    // thread_local! {
    //     pub static TID: u64 = TIDS.fetch_add(1);
    // }
    // fn thread_indent()->String{
    //     " ".repeat(TID*);
    // }

    struct CheckedSemaphore {
        semaphore: Semaphore,
        counter: Mutex<u64>,
    }

    impl CheckedSemaphore {
        fn new(capacity: u64) -> Self {
            CheckedSemaphore {
                semaphore: Semaphore::new(capacity),
                counter: Mutex::new(0),
            }
        }
        async fn acquire(&self, amount: u64) -> Result<SemaphoreGuard<'_>, LockError> {
            //println!("+ {}", amount);
            let guard = self.semaphore.acquire(amount).await?;
            let mut lock = self.counter.lock().unwrap();
            //println!("{} + {} = {} ", *lock, amount, *lock + amount);
            *lock += amount;
            assert!(*lock <= self.semaphore.capacity());
            mem::drop(lock);
            //println!("{:?}", self.semaphore);
            Ok(guard)
        }
        fn release(&self, amount: u64) -> Result<(), Underflow> {
            let mut lock = self.counter.lock().unwrap();
            assert!(*lock >= amount);
            //println!("{} - {} = {} ", *lock, amount, *lock - amount);
            *lock -= amount;
            mem::drop(lock);
            let result = self.semaphore.release(amount);
            //println!("{:?}", self.semaphore);
            result
        }
    }

    #[test]
    fn test_multicore() {
        let capacity = 100;
        let semaphore = Arc::new(CheckedSemaphore::new(capacity));
        let pool = ThreadPool::builder().pool_size(10).create().unwrap();
        (0..100).map(|_thread|
            pool.spawn_with_handle({
                let semaphore = semaphore.clone();
                async move {
                    //let indent = " ".repeat(thread * 10);
                    let mut owned = 0;
                    for _i in 0..1000 {
                        //println!("{:?}", semaphore.semaphore);
                        if owned == 0 {
                            owned = thread_rng().gen_range(0, capacity + 1);
                            //println!("{} : acquiring {}", thread, owned);
                            semaphore.acquire(owned).await.unwrap().forget();
                        } else {
                            let mut rng = thread_rng();
                            let r = if rng.gen_bool(0.5) {
                                owned
                            } else {
                                rng.gen_range(1, owned + 1)
                            };
                            owned -= r;
                            semaphore.release(r).unwrap();
                        }
                    }
                    semaphore.release(owned).unwrap();
                }
            }).unwrap()
        ).collect::<Vec<_>>().into_iter().for_each(block_on);
        mem::drop(pool);
        assert_eq!(Arc::strong_count(&semaphore), 1);
    }
}
