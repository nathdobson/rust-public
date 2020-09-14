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
use crate::util::{assert_sync_send, clone_waker, yield_once};
use std::sync::atomic::Ordering::{Acquire, AcqRel};

pub struct Semaphore {
    state: Atomic<StatePacker>,
    front: UnsafeCell<*const Waiter>,
    free: Atomic<CastPacker<(*const Waiter, usize), AtomicUsize2>>,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct State {
    available: usize,
    back: *const Waiter,
    locked: bool,
    queued: bool,
}

pub struct Waiter {
    waker: Atomic<WakerPacker>,
    amount: UnsafeCell<usize>,
    next: Atomic<CastPacker<*const Waiter, AtomicUsize>>,
}

#[must_use = "If unused the acquire will be a noop."]
pub struct SemaphoreGuard<'a> {
    semaphore: &'a Semaphore,
    amount: usize,
}

struct WakerPacker;

struct StatePacker;

impl AtomicPacker for WakerPacker {
    type Impl = AtomicU128;
    type Value = Option<Waker>;

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

impl AtomicPacker for StatePacker {
    type Impl = AtomicUsize2;
    type Value = State;

    unsafe fn encode(state: State) -> usize2 {
        ((state.available as usize2) << (size_of::<usize>() * 8)) |
            (state.back as usize2) |
            (if state.locked { 2 } else { 0 }) |
            (if state.queued { 1 } else { 0 })
    }

    unsafe fn decode(x: u128) -> State {
        State {
            available: (x >> (size_of::<usize>() * 8)) as usize,
            back: ((x as usize) & (!3usize)) as *const Waiter,
            locked: (x & 2) == 2,
            queued: (x & 1) == 1,
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
                write!(f, " {:?} {:?}", *(*self.0).amount.get(), DebugPtr((*self.0).next.load(Ordering::SeqCst)))?;
            }
            Ok(())
        }
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
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

impl Debug for Semaphore {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            let mut w = f.debug_struct("Semaphore");
            let state = self.state.load(Ordering::SeqCst);
            w.field("A", &state.available);
            w.field("Q", &state.queued);
            w.field("L", &state.locked);
            w.field("B", &DebugPtr(state.back));
            w.field("F", &DebugPtr(*self.front.get()));
            let (free, free_gen) = self.free.load(Ordering::SeqCst);
            w.field("P", &(DebugPtr(free), free_gen));
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
                locked: false,
                queued: false,
            }),
            front: UnsafeCell::new(null()),
            free: Atomic::new((null(), 0)),
        }
    }

    pub fn available(&self) -> usize {
        self.state.load(Ordering::SeqCst).available
    }

    pub fn try_acquire(&self, amount: usize) -> Result<SemaphoreGuard<'_>, WouldBlock> {
        let mut state = self.state.transact_session(Acquire);
        loop {
            let mut state = state.transact();
            if amount > state.available || state.queued {
                return Err(WouldBlock);
            }
            state.available -= amount;
            if state.commit(AcqRel) {
                return Ok(SemaphoreGuard { semaphore: self, amount });
            }
        }
    }

    unsafe fn allocate(&self) -> *const Waiter {
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

    unsafe fn free(&self, waiter: *const Waiter) {
        (*waiter).waker.swap(None, Ordering::Relaxed);
        self.free.fetch_update(
            Ordering::AcqRel, Ordering::Acquire,
            |(free, free_ver)| {
                (*waiter).next.store(free, Ordering::Relaxed);
                Some((waiter, free_ver + 1))
            }).unwrap();
    }

    pub fn acquire(&self, amount: usize) -> impl Future<Output=SemaphoreGuard<'_>> {
        unsafe { assert_sync_send(self.acquire_impl(amount)) }
    }

    unsafe fn try_acquire_or_enqueue(&self, amount: usize, waiter: *const Waiter) -> bool {
        let mut acquired = false;
        self.state.fetch_update(
            Ordering::AcqRel, Ordering::Acquire,
            |state| {
                if amount <= state.available && !state.queued {
                    acquired = true;
                    Some(State { available: state.available - amount, ..state })
                } else {
                    acquired = false;
                    (*waiter).next.store(state.back, Ordering::Relaxed);
                    Some(State { queued: true, back: waiter, ..state })
                }
            }).unwrap();
        if acquired {
            self.free(waiter);
        }
        acquired
    }


    unsafe fn pop_and_unlock(&self) {
        let old = *self.front.get();
        *self.front.get() = (*old).next.load(Ordering::Relaxed);
        self.free(old);
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
        self.unlock();
    }

    unsafe fn try_acquire_pop_and_unlock(&self) -> bool {
        let waiter = *self.front.get();
        let amount = *(*waiter).amount.get();
        let next = (*waiter).next.load(Ordering::Relaxed);
        let mut acquired = false;
        self.state.fetch_update(
            Ordering::AcqRel, Ordering::Acquire,
            |state| {
                if amount <= state.available {
                    *self.front.get() = next;
                    acquired = true;
                    Some(State {
                        available: state.available - amount,
                        back: state.back,
                        locked: true,
                        queued: next != null() || state.back != null(),
                    })
                } else {
                    *self.front.get() = waiter;
                    acquired = false;
                    Some(State { locked: false, ..state })
                }
            },
        ).unwrap();
        if acquired {
            self.unlock();
            self.free(waiter);
        }
        acquired
    }

    async unsafe fn acquire_impl(&self, amount: usize) -> SemaphoreGuard<'_> {
        if let Ok(guard) = self.try_acquire(amount) {
            return guard;
        }
        let waiter = self.allocate();
        (*(*waiter).amount.get()) = amount;
        (*waiter).waker.swap(Some(clone_waker().await), Ordering::Relaxed);
        if self.try_acquire_or_enqueue(amount, waiter) {
            return SemaphoreGuard { semaphore: self, amount };
        }
        loop {
            yield_once(|| {
                let old = (*waiter).waker.swap(None, Ordering::AcqRel);
                if old.is_none() {
                    assert_eq!(*self.front.get(), waiter);
                    self.pop_and_unlock();
                }
            }).await;
            let old = (*waiter).waker.swap(Some(clone_waker().await), Ordering::AcqRel);
            if old.is_some() {
                continue;
            }
            assert_eq!(*self.front.get(), waiter);
            if self.try_acquire_pop_and_unlock() {
                return SemaphoreGuard { semaphore: self, amount };
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
            }
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

    pub fn release(&self, amount: usize) {
        unsafe {
            if !self.state.fetch_update(
                Ordering::AcqRel, Ordering::Acquire,
                |state| {
                    Some(State {
                        available: state.available + amount,
                        locked: true,
                        ..state
                    })
                }).unwrap().locked {
                self.unlock();
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
        let pool = ThreadPool::builder().pool_size(10).create().unwrap();
        (0..30).map(|_thread|
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
