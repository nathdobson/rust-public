use std::{mem, fmt};
use std::task::{Waker, Poll, Context};
use std::cell::RefCell;
use std::future::Future;
use crate::queue::{Queue, QueueKey};
use std::sync::{Mutex, MutexGuard};
use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::sync::atomic::Ordering::AcqRel;
use crate::{ReleaseGuard, WouldBlock, AcquireRelease};

pub struct SemaphoreImpl { inner: Mutex<SemaphoreInner> }

#[derive(Debug)]
struct Waiter {
    waker: Waker,
    amount: usize,
}

#[derive(Debug)]
struct SemaphoreInner {
    available: usize,
    waiting: Queue<Waiter>,
}

pub struct AcquireImpl {
    amount: usize,
    mode: AcquireMode,
}

#[derive(Clone, Copy)]
pub enum AcquireMode {
    Enter,
    Waiting(QueueKey),
    Poison,
}

impl SemaphoreInner {
    fn new(initial: usize) -> Self {
        SemaphoreInner {
            available: initial,
            waiting: Queue::new(),
        }
    }
}

impl Debug for SemaphoreImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.inner.lock().unwrap())?;
        Ok(())
    }
}

impl SemaphoreImpl {
    fn release_impl<'a>(&'a self, mut this: MutexGuard<'a, SemaphoreInner>, amount: usize) {
        this.available += amount;
        while let Some(front) = this.waiting.front() {
            if front.amount > this.available {
                break;
            }
            this.available -= front.amount;
            let waker = this.waiting.pop_front().unwrap().waker;
            mem::drop(this);
            waker.wake();
            this = self.inner.lock().unwrap();
        }
        mem::drop(this);
    }

}

impl AcquireRelease for SemaphoreImpl {
    type Acq = AcquireImpl;

    fn new(initial: usize) -> Self {
        SemaphoreImpl {
            inner: Mutex::new(SemaphoreInner::new(initial))
        }
    }

    unsafe fn acquire_new(&self, amount: usize) -> Self::Acq {
        AcquireImpl {
            amount,
            mode: AcquireMode::Enter,
        }
    }

    unsafe fn acquire_poll(&self, mut acq: Pin<&mut Self::Acq>, cx: &mut Context) -> Poll<()> {
        match acq.mode {
            AcquireMode::Enter => {
                let mut inner = self.inner.lock().unwrap();
                if acq.amount <= inner.available && inner.waiting.front().is_none() {
                    inner.available -= acq.amount;
                    acq.mode = AcquireMode::Poison;
                    return Poll::Ready(());
                }
                let id = inner.waiting.push_back(Waiter {
                    waker: cx.waker().clone(),
                    amount: acq.amount,
                });
                acq.mode = AcquireMode::Waiting(id);
                return Poll::Pending;
            }
            AcquireMode::Waiting(id) => {
                let mut inner = self.inner.lock().unwrap();
                if id < inner.waiting.front_key() {
                    acq.mode = AcquireMode::Poison;
                    return Poll::Ready(());
                } else {
                    let call = inner.waiting.get_mut(id).unwrap();
                    call.waker = cx.waker().clone();
                    return Poll::Pending;
                }
            }
            AcquireMode::Poison => unreachable!()
        }
    }

    unsafe fn acquire_drop(&self, acq: Pin<&mut Self::Acq>) {
        match acq.mode {
            AcquireMode::Waiting(id) => {
                let mut this = self.inner.lock().unwrap();
                match this.waiting.get_mut(id) {
                    None => self.release_impl(this, acq.amount),
                    Some(call) => call.amount = 0,
                }
            }
            AcquireMode::Enter { .. } => {}
            AcquireMode::Poison => {}
        }
    }

    fn try_acquire(&self, amount: usize) -> Result<(), WouldBlock> {
        let mut this = self.inner.lock().unwrap();
        if amount <= this.available && this.waiting.front().is_none() {
            this.available -= amount;
            Ok(())
        } else {
            Err(WouldBlock)
        }
    }

    fn release(&self, amount: usize) {
        self.release_impl(self.inner.lock().unwrap(), amount);
    }
}
//
// #[cfg(test)]
// mod test {
//     use rand_xorshift::XorShiftRng;
//     use std::{mem, thread};
//     use futures::executor::{LocalPool, block_on, ThreadPool};
//     use std::rc::Rc;
//     use futures::task::{LocalSpawnExt, SpawnExt, LocalFutureObj, FutureObj};
//     use rand::{SeedableRng, Rng, thread_rng};
//     use std::time::{Duration, Instant};
//     use async_std::future::{timeout, TimeoutError};
//     use async_std::task::sleep;
//     use std::cell::RefCell;
//     use defer::defer;
//     use futures::future::poll_fn;
//     use futures::{Future, StreamExt};
//     use futures::future::pending;
//     use std::task::Poll;
//     use futures::poll;
//     use std::process::abort;
//     use futures::stream::FuturesUnordered;
//     use crate::shared_mutex::{SemaphoreImpl};
//     use std::sync::{Arc, Mutex};
//     use crate::{WouldBlock, ReleaseGuard};
//
//     #[test]
//     fn test_random() {
//         let mut pool = LocalPool::new();
//         let spawner = pool.spawner();
//         let semaphore = Rc::new(SemaphoreImpl::new(10));
//         for i in 0usize..100 {
//             spawner.spawn_local({
//                 let mut rng = XorShiftRng::seed_from_u64((i + 1000) as u64);
//                 let semaphore = semaphore.clone();
//                 async move {
//                     let indent = " ".repeat(i);
//                     println!("{}A", indent);
//                     let t = Duration::from_millis(rng.gen_range(0, 10) * 10);
//                     match timeout(t, semaphore.acquire(1)).await {
//                         Ok(guard) => {
//                             println!("{}B", indent);
//                             let time = rng.gen_range(0, 10);
//                             sleep(Duration::from_millis(time)).await;
//                             println!("{}C", indent);
//                             mem::drop(guard);
//                             println!("{}D", indent);
//                         }
//                         Err(_) => {
//                             println!("{}E", indent);
//                         }
//                     }
//                 }
//             }).unwrap();
//         }
//         pool.run();
//     }
//
//     #[test]
//     fn test_empty() {
//         let semaphore = SemaphoreImpl::new(0);
//         block_on(semaphore.acquire(0));
//         assert!(semaphore.try_acquire(1).contains_err(&WouldBlock));
//     }
//
//     #[test]
//     fn test_shared() {
//         let semaphore = Rc::new(SemaphoreImpl::new(10));
//         let g1 = block_on(semaphore.acquire(5));
//         let _g2 = block_on(semaphore.acquire(5));
//         let mut pool = LocalPool::new();
//         let spawner = pool.spawner();
//         let finished = Rc::new(RefCell::new(false));
//         spawner.spawn_local({
//             let finished = finished.clone();
//             let semaphore = semaphore.clone();
//             async move {
//                 println!("A");
//                 semaphore.acquire(5).await;
//                 println!("B");
//                 *finished.borrow_mut() = true;
//             }
//         }).unwrap();
//         pool.run_until_stalled();
//         assert!(!*finished.borrow());
//         mem::drop(g1);
//         pool.run_until_stalled();
//         assert!(*finished.borrow());
//     }
//
//     #[test]
//     fn test_interrupt() {
//         let semaphore = Rc::new(SemaphoreImpl::new(10));
//         println!("A");
//         let _g1 = block_on(semaphore.acquire(5));
//         println!("B");
//         let g2 = block_on(semaphore.acquire(5));
//         println!("C");
//         let mut pool = LocalPool::new();
//         let spawner = pool.spawner();
//         spawner.spawn_local({
//             let semaphore = semaphore.clone();
//             async move {
//                 println!("D");
//                 semaphore.acquire(5).await;
//                 println!("E");
//             }
//         }).unwrap();
//         println!("F");
//         pool.run_until_stalled();
//         println!("G");
//         mem::drop(g2);
//         println!("H");
//         mem::drop(spawner);
//         println!("I");
//         mem::drop(pool);
//         println!("J");
//         let _g3 = block_on(semaphore.acquire(5));
//         println!("K");
//     }
//
//     struct CheckedSemaphore {
//         capacity: usize,
//         semaphore: SemaphoreImpl,
//         counter: Mutex<usize>,
//     }
//
//     impl CheckedSemaphore {
//         fn new(capacity: usize) -> Self {
//             CheckedSemaphore {
//                 capacity,
//                 semaphore: SemaphoreImpl::new(capacity),
//                 counter: Mutex::new(0),
//             }
//         }
//         async fn acquire(&self, amount: usize) -> ReleaseGuard<&SemaphoreImpl, SemaphoreImpl> {
//             //println!("+ {}", amount);
//             let guard = self.semaphore.acquire(amount).await;
//             let mut lock = self.counter.lock().unwrap();
//             //println!("{} + {} = {} ", *lock, amount, *lock + amount);
//             *lock += amount;
//             assert!(*lock <= self.capacity);
//             mem::drop(lock);
//             //println!("{:?}", self.semaphore);
//             guard
//         }
//         fn release(&self, amount: usize) {
//             let mut lock = self.counter.lock().unwrap();
//             assert!(*lock >= amount);
//             //println!("{} - {} = {} ", *lock, amount, *lock - amount);
//             *lock -= amount;
//             mem::drop(lock);
//             let result = self.semaphore.release(amount);
//             //println!("{:?}", self.semaphore);
//             result
//         }
//     }
//
//     #[test]
//     fn test_multicore() {
//         let capacity = 100;
//         let semaphore = Arc::new(CheckedSemaphore::new(capacity));
//         let pool = ThreadPool::builder().pool_size(2).create().unwrap();
//         (0..2).map(|_thread|
//             pool.spawn_with_handle({
//                 let semaphore = semaphore.clone();
//                 async move {
//                     //let indent = " ".repeat(thread * 10);
//                     let mut owned = 0;
//                     for _i in 0..500 {
//                         //println!("{:?}", semaphore.semaphore);
//                         if owned == 0 {
//                             owned = thread_rng().gen_range(0, capacity + 1);
//                             //println!("{} : acquiring {}", thread, owned);
//                             let dur = Duration::from_millis(thread_rng().gen_range(0, 10));
//                             if let Ok(guard) =
//                             timeout(dur, semaphore.acquire(owned)).await {
//                                 guard.forget();
//                             } else {
//                                 owned = 0;
//                             }
//                         } else {
//                             let mut rng = thread_rng();
//                             let r = if rng.gen_bool(0.5) {
//                                 owned
//                             } else {
//                                 rng.gen_range(1, owned + 1)
//                             };
//                             owned -= r;
//                             semaphore.release(r);
//                             //println!("{} : released {}", thread, owned);
//                         }
//                     }
//                     semaphore.release(owned);
//                 }
//             }).unwrap()
//         ).collect::<Vec<_>>().into_iter().for_each(block_on);
//         mem::drop(pool);
//         assert_eq!(Arc::strong_count(&semaphore), 1);
//     }
// }