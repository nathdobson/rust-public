use std::sync::atomic::{Ordering, AtomicU128, AtomicUsize};
use std::{mem, fmt};
use crate::{WouldBlock};
use std::marker::PhantomData;
use std::ptr::null;
use std::mem::{MaybeUninit, size_of};
use std::cell::UnsafeCell;
use std::task::Waker;
use std::sync::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::ops::DerefMut;
use std::fmt::{Debug, Formatter};
use crate::atomic::{Atomic, AtomicPacker, CastPacker, AtomicUsize2, AtomicInteger, usize2};
use crate::util::{assert_sync_send};
use std::sync::atomic::Ordering::{SeqCst, Relaxed, AcqRel, Acquire, Release};
use crate::shared::Mode::{OPEN, QUEUED, LOCKED};
use crate::waker::{WakerPacker, AtomicWaker};
use crate::freelist::FreeList;

pub struct Semaphore {
    state: Atomic<StatePacker>,
    front: UnsafeCell<*const Waiter>,
    freelist: FreeList<Waiter>,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
enum Mode {
    OPEN,
    QUEUED,
    LOCKED,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct State {
    available: usize,
    back: *const Waiter,
    mode: Mode,
}

#[repr(align(64))]
pub struct Waiter {
    waker: AtomicWaker,
    next: UnsafeCell<*const Waiter>,
}

#[must_use = "If unused the acquire will be a noop."]
pub struct SemaphoreGuard<'a> {
    semaphore: &'a Semaphore,
    amount: usize,
}

struct StatePacker;

impl AtomicPacker for StatePacker {
    type Impl = AtomicUsize2;
    type Value = State;

    unsafe fn encode(state: State) -> usize2 {
        ((state.available as usize2) << (size_of::<usize>() * 8)) |
            (state.back as usize2) |
            (match state.mode {
                Mode::OPEN => 0,
                Mode::QUEUED => 1,
                Mode::LOCKED => 2,
            })
    }

    unsafe fn decode(x: u128) -> State {
        State {
            available: (x >> (size_of::<usize>() * 8)) as usize,
            back: ((x as usize) & (!3usize)) as *const Waiter,
            mode: match x & 3 {
                0 => Mode::OPEN,
                1 => Mode::QUEUED,
                2 => Mode::LOCKED,
                _ => unreachable!(),
            },
        }
    }
}

unsafe impl Sync for Semaphore {}

unsafe impl Send for Semaphore {}

impl SemaphoreGuard<'_> {
    pub fn forget(self) {
        mem::forget(self)
    }
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        self.semaphore.release(self.amount);
    }
}

struct DebugPtr(*const Waiter);

impl Debug for DebugPtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            write!(f, "{:?}", self.0)?;
            if self.0 != null() {
                write!(f, " {:?}", DebugPtr(*(*self.0).next.get()))?;
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
    pub fn new(initial: usize) -> Semaphore {
        Semaphore {
            state: Atomic::new(State {
                available: initial,
                back: null(),
                mode: OPEN,
            }),
            front: UnsafeCell::new(null()),
            freelist: FreeList::new(),
        }
    }

    pub fn available(&self) -> usize {
        self.state.load(SeqCst).available
    }

    pub fn try_acquire(&self, amount: usize) -> Result<SemaphoreGuard<'_>, WouldBlock> {
        let mut old_state = self.state.load(Relaxed);
        loop {
            let mut state = old_state;
            if amount > state.available || state.mode != OPEN {
                return Err(WouldBlock);
            }
            state.available -= amount;
            if self.state.compare_update_weak(
                &mut old_state, state, AcqRel, Relaxed) {
                return Ok(SemaphoreGuard { semaphore: self, amount });
            }
        }
    }

    pub fn acquire(&self, amount: usize) -> impl Future<Output=SemaphoreGuard<'_>> {
        unsafe { assert_sync_send(self.acquire_impl(amount)) }
    }

    async unsafe fn acquire_impl(&self, amount: usize) -> SemaphoreGuard<'_> {
        if let Ok(guard) = self.try_acquire(amount) {
            return guard;
        }
        let waiter =
            self.freelist.allocate(Waiter {
                waker: AtomicWaker::new().await,
                next: UnsafeCell::new(null()),
            });
        if self.try_acquire_or_push(amount, waiter) {
            return SemaphoreGuard { semaphore: self, amount };
        }
        loop {
            (*waiter).waker.wait(|woken: bool| {
                if woken {
                    assert_eq!(*self.front.get(), waiter);
                    self.pop_and_unlock();
                }
            }).await;
            assert_eq!(*self.front.get(), waiter);
            if self.try_acquire_pop_and_unlock(amount) {
                return SemaphoreGuard { semaphore: self, amount };
            }
        }
    }

    unsafe fn try_acquire_or_push(&self, amount: usize, waiter: *const Waiter) -> bool {
        let mut old_state = self.state.load(Acquire);
        loop {
            let mut state = old_state;
            if amount < state.available && state.mode == OPEN {
                state.available -= amount;
                if self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    self.freelist.free(waiter);
                    return true;
                }
            } else {
                *(*waiter).next.get() = state.back;
                if state.mode == OPEN {
                    state.mode = QUEUED;
                }
                state.back = waiter;
                if self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    return false;
                }
            }
        }
    }

    unsafe fn try_acquire_pop_and_unlock(&self, amount: usize) -> bool {
        let waiter = *self.front.get();
        let next = *(*waiter).next.get();
        let mut old_state = self.state.load(Acquire);
        loop {
            let mut state = old_state;
            if amount <= state.available {
                *self.front.get() = next;
                state.available -= amount;
                if self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    self.freelist.free(waiter);
                    self.unlock();
                    return true;
                }
            } else {
                *self.front.get() = waiter;
                state.mode = QUEUED;
                if self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    return false;
                }
            }
        }
    }

    unsafe fn pop_and_unlock(&self) {
        let old = *self.front.get();
        *self.front.get() = *(*old).next.get();
        self.freelist.free(old);
        self.unlock();
    }

    pub fn release(&self, amount: usize) {
        unsafe {
            let mut old_state = self.state.load(Acquire);
            loop {
                let mut state = old_state;
                state.available += amount;
                if state.mode == LOCKED {
                    if self.state.compare_update_weak(
                        &mut old_state, state, AcqRel, Acquire) {
                        return;
                    }
                } else {
                    state.mode = LOCKED;
                    if self.state.compare_update_weak(
                        &mut old_state, state, AcqRel, Acquire) {
                        self.unlock();
                        return;
                    }
                }
            }
        }
    }

    unsafe fn unlock(&self) {
        let mut old_state = self.state.load(Acquire);
        loop {
            let mut state = old_state;
            if *self.front.get() == null() {
                if state.back == null() {
                    state.mode = OPEN;
                    if !self.state.compare_update_weak(
                        &mut old_state, state, AcqRel, Acquire) {
                        continue;
                    }
                    return;
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
            if (*front).waker.notify() {
                return;
            } else {
                *self.front.get() = *(*front).next.get();
                self.freelist.free(front);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::shared::{Semaphore, SemaphoreGuard};
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

    #[test]
    fn test_simple() {
        let semaphore = Rc::new(Semaphore::new(10));
        let mut pool = LocalPool::new();
        let spawner = pool.spawner();
        spawner.spawn_local({
            let semaphore = semaphore.clone();
            async move {
                semaphore.acquire(10).await.forget();
                semaphore.acquire(10).await.forget();
            }
        }).unwrap();
        pool.run_until_stalled();
        semaphore.release(10);
        pool.run();
    }

    struct CheckedSemaphore {
        capacity: usize,
        semaphore: Semaphore,
        counter: Mutex<usize>,
    }

    impl CheckedSemaphore {
        fn new(capacity: usize) -> Self {
            CheckedSemaphore {
                capacity,
                semaphore: Semaphore::new(capacity),
                counter: Mutex::new(0),
            }
        }
        async fn acquire(&self, amount: usize) -> SemaphoreGuard<'_> {
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
        fn release(&self, amount: usize) {
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
        let pool = ThreadPool::builder().pool_size(2).create().unwrap();
        (0..2).map(|_thread|
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
