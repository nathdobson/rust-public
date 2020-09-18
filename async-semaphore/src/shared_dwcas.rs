use std::sync::atomic::{Ordering, AtomicU128, AtomicUsize};
use std::{mem, fmt};
use crate::{WouldBlock, ReleaseGuard, Releaser};
use std::marker::PhantomData;
use std::ptr::null;
use std::mem::{MaybeUninit, size_of};
use std::cell::UnsafeCell;
use std::task::{Waker, Context, Poll};
use std::sync::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::ops::DerefMut;
use std::fmt::{Debug, Formatter};
use crate::atomic::{Atomic, AtomicPacker, CastPacker, AtomicUsize2, AtomicInteger, usize2};
use std::sync::atomic::Ordering::{SeqCst, Relaxed, AcqRel, Acquire, Release};
use crate::shared_dwcas::Mode::{Open, Queued, Locked, LockedDirty};
use crate::waker::{WakerPacker, WakerValue};
use crate::freelist::FreeList;
use crate::waker::AtomicWaker;
use std::hint::unreachable_unchecked;
use crate::waker::AtomicWaker::Waiting;
use crate::waker::AtomicWaker::Waking;
use crate::waker::AtomicWaker::Cancelled;

pub struct Semaphore {
    state: Atomic<StatePacker>,
    front: UnsafeCell<*const Waiter>,
    freelist: FreeList<Waiter>,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
enum Mode {
    Open,
    Queued,
    Locked,
    LockedDirty,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct State {
    available: u64,
    back: *const Waiter,
    mode: Mode,
}

#[repr(align(64))]
pub struct Waiter {
    waker: Atomic<WakerPacker>,
    next: UnsafeCell<*const Waiter>,
    amount: u64,
}

struct StatePacker;

pub struct AcquireImpl<'a>(AcquireImplInner<'a>);

#[derive(Clone, Copy)]
pub enum AcquireImplInner<'a> {
    Enter { semaphore: &'a Semaphore, amount: u64 },
    Waiting { semaphore: &'a Semaphore, waiter: *const Waiter, amount: u64 },
    Poison,
}

impl AtomicPacker for StatePacker {
    type Impl = AtomicUsize2;
    type Value = State;

    unsafe fn encode(state: State) -> usize2 {
        ((state.available as usize2) << (size_of::<usize>() * 8)) |
            (state.back as usize2) |
            (match state.mode {
                Open => 0,
                Queued => 1,
                Locked => 2,
                LockedDirty => 3,
            })
    }

    unsafe fn decode(x: u128) -> State {
        State {
            available: (x >> (size_of::<u64>() * 8)) as u64,
            back: ((x as usize) & (!3usize)) as *const Waiter,
            mode: match x & 3 {
                0 => Open,
                1 => Queued,
                2 => Locked,
                3 => LockedDirty,
                _ => unreachable!(),
            },
        }
    }
}

unsafe impl Sync for Semaphore {}

unsafe impl Send for Semaphore {}

unsafe impl<'a> Sync for AcquireImpl<'a> {}

unsafe impl<'a> Send for AcquireImpl<'a> {}

struct DebugPtr(*const Waiter);

impl Debug for DebugPtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            write!(f, "{:?}", self.0)?;
            if self.0 != null() {
                write!(f, " {:?} {:?} {:?}", (*self.0).amount, (*self.0).waker.load(Ordering::Relaxed), DebugPtr(*(*self.0).next.get()))?;
            }
            Ok(())
        }
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            for &(mut ptr) in &[*self.front.get(),
                self.state.load(Relaxed).back] {
                while ptr != null() {
                    let next = *(*ptr).next.get();
                    self.freelist.free(ptr);
                    ptr = next;
                }
            }
        }
    }
}

impl Debug for Semaphore {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            let mut w = f.debug_struct("Semaphore");
            let state = self.state.load(SeqCst);
            w.field("A", &state.available);
            w.field("M", &state.mode);
            w.field("B", &DebugPtr(state.back));
            w.field("F", &DebugPtr(*self.front.get()));
            w.finish()
        }
    }
}

impl Semaphore {
    pub fn available(&self) -> u64 {
        self.state.load(SeqCst).available
    }

    unsafe fn try_acquire_or_push(&self, amount: u64, waiter: *const Waiter) -> bool {
        let mut old_state = self.state.load(Acquire);
        loop {
            let mut state = old_state;
            if amount <= state.available && state.mode == Open {
                state.available -= amount;
                if self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    self.freelist.free(waiter);
                    return true;
                }
            } else {
                *(*waiter).next.get() = state.back;
                if state.mode == Open {
                    state.mode = Queued;
                }
                state.back = waiter;
                if self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    return false;
                }
            }
        }
    }

    // unsafe fn try_acquire_pop_and_unlock(&self, amount: u64) -> bool {
    //     let waiter = *self.front.get();
    //     let next = *(*waiter).next.get();
    //     let mut old_state = self.state.load(Acquire);
    //     loop {
    //         let mut state = old_state;
    //         if amount <= state.available {
    //             *self.front.get() = next;
    //             state.available -= amount;
    //             if self.state.compare_update_weak(
    //                 &mut old_state, state, AcqRel, Acquire) {
    //                 self.freelist.free(waiter);
    //                 self.unlock();
    //                 return true;
    //             }
    //         } else {
    //             *self.front.get() = waiter;
    //             state.mode = QUEUED;
    //             if self.state.compare_update_weak(
    //                 &mut old_state, state, AcqRel, Acquire) {
    //                 return false;
    //             }
    //         }
    //     }
    // }

    // unsafe fn pop_and_unlock(&self) {
    //     let old = *self.front.get();
    //     *self.front.get() = *(*old).next.get();
    //     self.freelist.free(old);
    //     self.unlock();
    // }

    unsafe fn unlock(&self) {
        let mut old_state = self.state.load(Acquire);
        loop {
            let mut state = old_state;
            if state.mode == LockedDirty {
                state.mode = Locked;
                if !self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    continue;
                }
            }

            if *self.front.get() == null() {
                if state.back == null() {
                    state.mode = Open;
                    if !self.state.compare_update_weak(
                        &mut old_state, state, AcqRel, Acquire) {
                        continue;
                    }
                    break;
                }
                state.back = null();
                let mut back = old_state.back;
                if !self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    continue;
                }
                while back != null() {
                    let next = *((*back).next.get());
                    *(*back).next.get() = *self.front.get();
                    *self.front.get() = back;
                    back = next;
                }
                assert_ne!((*self.front.get()), null());
                continue;
            }
            let front = *self.front.get();
            if (*front).amount <= state.available {
                *self.front.get() = *(*front).next.get();
                match (*front).waker.swap(AtomicWaker::Waking, Ordering::AcqRel) {
                    Cancelled => {
                        self.freelist.free(front);
                        continue;
                    }
                    Waiting(waker) => {
                        loop {
                            state = old_state;
                            state.available -= (*front).amount;
                            if self.state.compare_update_weak(
                                &mut old_state, state, AcqRel, Acquire) {
                                break;
                            }
                        }
                        WakerValue::decode(waker).wake();
                        continue;
                    }
                    Waking => unreachable!()
                }
            }
            match (*front).waker.load(Ordering::Acquire) {
                AtomicWaker::Cancelled => {
                    *self.front.get() = *(*front).next.get();
                    self.freelist.free(front);
                    continue;
                }
                AtomicWaker::Waiting(_waker) => {}
                AtomicWaker::Waking => unreachable!(),
            }
            state.mode = Queued;
            if self.state.compare_update_weak(
                &mut old_state, state, AcqRel, Acquire) {
                break;
            }
        }
    }

    pub fn new(initial: u64) -> Self {
        Semaphore {
            state: Atomic::new(State {
                available: initial,
                back: null(),
                mode: Open,
            }),
            front: UnsafeCell::new(null()),
            freelist: FreeList::new(),
        }
    }

    fn try_acquire(&self, amount: u64) -> Result<ReleaseGuard<&Self, Self>, WouldBlock> {
        let mut old_state = self.state.load(Acquire);
        loop {
            let mut state = old_state;
            if amount > state.available || state.mode != Open {
                return Err(WouldBlock);
            }
            state.available -= amount;
            if self.state.compare_update_weak(
                &mut old_state, state, AcqRel, Acquire) {
                return Ok(ReleaseGuard::new(self, amount));
            }
        }
    }

    pub fn acquire(&self, amount: u64) -> AcquireImpl<'_> {
        AcquireImpl(AcquireImplInner::Enter { semaphore: self, amount })
    }
}


impl Releaser for Semaphore {
    fn release(&self, amount: u64) {
        unsafe {
            let mut old_state = self.state.load(Acquire);
            loop {
                let mut state = old_state;
                state.available += amount;
                match state.mode {
                    Locked | LockedDirty => {
                        if self.state.compare_update_weak(
                            &mut old_state, state, AcqRel, Acquire) {
                            return;
                        }
                    }
                    Queued => {
                        state.mode = Locked;
                        if self.state.compare_update_weak(
                            &mut old_state, state, AcqRel, Acquire) {
                            self.unlock();
                            return;
                        }
                    }
                    Open => {
                        assert_eq!(state.back, null());
                        if self.state.compare_update_weak(
                            &mut old_state, state, AcqRel, Acquire) {
                            return;
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Future for AcquireImpl<'a> {
    type Output = ReleaseGuard<&'a Semaphore, Semaphore>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            let this = self.get_unchecked_mut();
            match this.0 {
                AcquireImplInner::Enter { semaphore, amount } => {
                    if let Ok(guard) = semaphore.try_acquire(amount) {
                        this.0 = AcquireImplInner::Poison;
                        return Poll::Ready(guard);
                    }
                    let waiter =
                        semaphore.freelist.allocate(Waiter {
                            waker: Atomic::new(AtomicWaker::Waiting(WakerValue::encode(cx.waker().clone()))),
                            next: UnsafeCell::new(null()),
                            amount,
                        });
                    if semaphore.try_acquire_or_push(amount, waiter) {
                        this.0 = AcquireImplInner::Poison;
                        return Poll::Ready(ReleaseGuard::new(semaphore, amount));
                    }
                    this.0 = AcquireImplInner::Waiting { semaphore, waiter, amount };
                    return Poll::Pending;
                }
                AcquireImplInner::Waiting { semaphore, waiter, amount } => {
                    match (*waiter).waker.swap(AtomicWaker::Waiting(WakerValue::encode(cx.waker().clone())), AcqRel) {
                        AtomicWaker::Waking => {}
                        AtomicWaker::Waiting(waker) => {
                            mem::drop(WakerValue::decode(waker));
                            return Poll::Pending;
                        }
                        AtomicWaker::Cancelled => {
                            println!("Unreachable {:?} for {:?}", semaphore, DebugPtr(waiter));
                            unreachable!()
                        }
                    }
                    semaphore.freelist.free(waiter);
                    return Poll::Ready(ReleaseGuard::new(semaphore, amount));
                }
                AcquireImplInner::Poison => unreachable!()
            }
        }
    }
}

impl<'a> Drop for AcquireImpl<'a> {
    fn drop(&mut self) {
        unsafe {
            match self.0 {
                AcquireImplInner::Waiting { semaphore, waiter, amount: _ } => {
                    match (*waiter).waker.swap(AtomicWaker::Cancelled, AcqRel) {
                        AtomicWaker::Waking => {
                            semaphore.release((*waiter).amount);
                            semaphore.freelist.free(waiter);
                        }
                        AtomicWaker::Waiting(waker) => {
                            mem::drop(WakerValue::decode(waker));
                            let mut old_state = semaphore.state.load(Acquire);
                            loop {
                                let mut state = old_state;
                                match state.mode {
                                    Open | Queued => {
                                        state.mode = Locked;
                                        if semaphore.state.compare_update_weak(
                                            &mut old_state, state, AcqRel, Acquire) {
                                            semaphore.unlock();
                                            break;
                                        }
                                    }
                                    Locked => {
                                        state.mode = LockedDirty;
                                        if semaphore.state.compare_update_weak(
                                            &mut old_state, state, AcqRel, Acquire) {
                                            break;
                                        }
                                    }
                                    LockedDirty => {
                                        break;
                                    }
                                }
                            }
                        }
                        AtomicWaker::Cancelled => unreachable!(),
                    }
                }
                AcquireImplInner::Enter { .. } => {}
                AcquireImplInner::Poison => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shared_dwcas::{Semaphore};
    use futures::executor::{LocalPool, ThreadPool, block_on};
    use futures::task::{LocalSpawnExt, SpawnExt};
    use rand::{thread_rng, Rng, SeedableRng};
    use std::sync::{Arc, Mutex};
    use std::rc::Rc;
    use std::cell::{Cell, RefCell};
    use rand_xorshift::XorShiftRng;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::mem;
    use futures::StreamExt;
    use itertools::Itertools;
    use std::time::Duration;
    use async_std::task::sleep;
    use async_std::future::timeout;
    use crate::{ReleaseGuard, Releaser};

    #[test]
    fn test_simple() {
        println!("A");
        let semaphore = Rc::new(Semaphore::new(10));
        println!("B");
        let mut pool = LocalPool::new();
        println!("C");
        let spawner = pool.spawner();
        println!("D");
        spawner.spawn_local({
            println!("E");
            let semaphore = semaphore.clone();
            async move {
                println!("F");
                semaphore.acquire(10).await.forget();
                println!("G");
                semaphore.acquire(10).await.forget();
                println!("H");
            }
        }).unwrap();
        println!("I");
        pool.run_until_stalled();
        println!("J");
        semaphore.release(10);
        println!("K");
        pool.run();
        println!("L");
    }

    struct CheckedSemaphore {
        capacity: u64,
        semaphore: Semaphore,
        counter: Mutex<u64>,
    }

    impl CheckedSemaphore {
        fn new(capacity: u64) -> Self {
            CheckedSemaphore {
                capacity,
                semaphore: Semaphore::new(capacity),
                counter: Mutex::new(0),
            }
        }
        async fn acquire(&self, amount: u64) -> ReleaseGuard<&Semaphore, Semaphore> {
            //println!("+ {}", amount);
            let guard = self.semaphore.acquire(amount).await;
            let mut lock = self.counter.lock().unwrap();
            //println!("{} + {} = {} ", *lock, amount, *lock + amount);
            *lock += amount;
            assert!(*lock <= self.capacity);
            mem::drop(lock);
            //println!("{:?}", self.semaphore);
            guard
        }
        fn release(&self, amount: u64) {
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
                    for _i in 0..500 {
                        //println!("{:?}", semaphore.semaphore);
                        if owned == 0 {
                            owned = thread_rng().gen_range(0, capacity + 1);
                            //println!("{} : acquiring {}", thread, owned);
                            let dur = Duration::from_millis(thread_rng().gen_range(0, 10));
                            if let Ok(guard) =
                            timeout(dur, semaphore.acquire(owned)).await {
                                guard.forget();
                            } else {
                                owned = 0;
                            }
                        } else {
                            let mut rng = thread_rng();
                            let r = if rng.gen_bool(0.5) {
                                owned
                            } else {
                                rng.gen_range(1, owned + 1)
                            };
                            owned -= r;
                            semaphore.release(r);
                            //println!("{} : released {}", thread, owned);
                        }
                    }
                    semaphore.release(owned);
                }
            }).unwrap()
        ).collect::<Vec<_>>().into_iter().for_each(block_on);
        mem::drop(pool);
        assert_eq!(Arc::strong_count(&semaphore), 1);
    }
}
