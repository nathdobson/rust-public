use std::{mem, fmt};
use std::task::{Waker, Poll, Context};
use std::cell::RefCell;
use std::future::Future;
use crate::queue::{Queue, QueueKey};
use crate::{WouldBlock, ReleaseGuard, Releaser};
use std::sync::{Mutex, MutexGuard};
use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::sync::atomic::Ordering::AcqRel;

pub struct Semaphore { inner: Mutex<SemaphoreInner> }

#[derive(Debug)]
struct Waiter {
    waker: Waker,
    amount: u64,
}

#[derive(Debug)]
struct SemaphoreInner {
    available: u64,
    waiting: Queue<Waiter>,
}

pub struct AcquireImpl<'a>(AcquireImplInner<'a>);

#[derive(Clone, Copy)]
pub enum AcquireImplInner<'a> {
    Enter { semaphore: &'a Semaphore, amount: u64 },
    Waiting { semaphore: &'a Semaphore, amount: u64, id: QueueKey },
    Poison,
}

impl SemaphoreInner {
    fn new(initial: u64) -> Self {
        SemaphoreInner {
            available: initial,
            waiting: Queue::new(),
        }
    }
}

impl Debug for Semaphore {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.inner.lock().unwrap())?;
        Ok(())
    }
}

impl Semaphore {
    fn release_impl<'a>(&'a self, mut this: MutexGuard<'a, SemaphoreInner>, amount: u64) {
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

    pub fn new(initial: u64) -> Self {
        Semaphore {
            inner: Mutex::new(SemaphoreInner::new(initial))
        }
    }

    pub fn try_acquire(&self, amount: u64) -> Result<ReleaseGuard<&Self, Self>, WouldBlock> {
        let mut this = self.inner.lock().unwrap();
        if amount <= this.available && this.waiting.front().is_none() {
            this.available -= amount;
            Ok(ReleaseGuard::new(self, amount))
        } else {
            Err(WouldBlock)
        }
    }

    pub fn acquire(&self, amount: u64) -> AcquireImpl {
        AcquireImpl(AcquireImplInner::Enter { semaphore: self, amount })
    }
}

impl Releaser for Semaphore {
    fn release(&self, amount: u64) {
        self.release_impl(self.inner.lock().unwrap(), amount);
    }
}

impl<'a> Future for AcquireImpl<'a> {
    type Output = ReleaseGuard<&'a Semaphore, Semaphore>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            let this = self.get_unchecked_mut();
            match mem::replace(&mut this.0, AcquireImplInner::Poison) {
                AcquireImplInner::Enter { semaphore, amount } => {
                    let mut inner = semaphore.inner.lock().unwrap();
                    if amount <= inner.available && inner.waiting.front().is_none() {
                        inner.available -= amount;
                        return Poll::Ready(ReleaseGuard::new(semaphore, amount));
                    }
                    let id = inner.waiting.push_back(Waiter {
                        waker: cx.waker().clone(),
                        amount,
                    });
                    this.0 = AcquireImplInner::Waiting { semaphore, amount, id };
                    return Poll::Pending;
                }
                AcquireImplInner::Waiting { semaphore, amount, id } => {
                    let mut inner = semaphore.inner.lock().unwrap();
                    if id < inner.waiting.front_key() {
                        return Poll::Ready(ReleaseGuard::new(semaphore, amount));
                    } else {
                        let call = inner.waiting.get_mut(id).unwrap();
                        call.waker = cx.waker().clone();
                        this.0 = AcquireImplInner::Waiting { semaphore, amount, id };
                        return Poll::Pending;
                    }
                }
                AcquireImplInner::Poison => unreachable!()
            }
        }
    }
}

impl<'a> Drop for AcquireImpl<'a> {
    fn drop(&mut self) {
        match self.0 {
            AcquireImplInner::Waiting { semaphore, amount, id } => {
                let mut this = semaphore.inner.lock().unwrap();
                match this.waiting.get_mut(id) {
                    None => semaphore.release_impl(this, amount),
                    Some(call) => call.amount = 0,
                }
            }
            AcquireImplInner::Enter { .. } => {}
            AcquireImplInner::Poison => {}
        }
    }
}

#[cfg(test)]
mod test {
    use rand_xorshift::XorShiftRng;
    use std::{mem, thread};
    use futures::executor::{LocalPool, block_on, ThreadPool};
    use std::rc::Rc;
    use futures::task::{LocalSpawnExt, SpawnExt, LocalFutureObj, FutureObj};
    use rand::{SeedableRng, Rng, thread_rng};
    use std::time::{Duration, Instant};
    use async_std::future::{timeout, TimeoutError};
    use async_std::task::sleep;
    use std::cell::RefCell;
    use crate::{WouldBlock, ReleaseGuard, Releaser};
    use defer::defer;
    use futures::future::poll_fn;
    use futures::{Future, StreamExt};
    use futures::future::pending;
    use std::task::Poll;
    use futures::poll;
    use std::process::abort;
    use futures::stream::FuturesUnordered;
    use crate::shared_mutex::{Semaphore};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_random() {
        let mut pool = LocalPool::new();
        let spawner = pool.spawner();
        let semaphore = Rc::new(Semaphore::new(10));
        for i in 0usize..100 {
            spawner.spawn_local({
                let mut rng = XorShiftRng::seed_from_u64((i + 1000) as u64);
                let semaphore = semaphore.clone();
                async move {
                    let indent = " ".repeat(i);
                    println!("{}A", indent);
                    let t = Duration::from_millis(rng.gen_range(0, 10) * 10);
                    match timeout(t, semaphore.acquire(1)).await {
                        Ok(guard) => {
                            println!("{}B", indent);
                            let time = rng.gen_range(0, 10);
                            sleep(Duration::from_millis(time)).await;
                            println!("{}C", indent);
                            mem::drop(guard);
                            println!("{}D", indent);
                        }
                        Err(_) => {
                            println!("{}E", indent);
                        }
                    }
                }
            }).unwrap();
        }
        pool.run();
    }

    #[test]
    fn test_empty() {
        let semaphore = Semaphore::new(0);
        block_on(semaphore.acquire(0));
        assert!(semaphore.try_acquire(1).contains_err(&WouldBlock));
    }

    #[test]
    fn test_shared() {
        let semaphore = Rc::new(Semaphore::new(10));
        let g1 = block_on(semaphore.acquire(5));
        let _g2 = block_on(semaphore.acquire(5));
        let mut pool = LocalPool::new();
        let spawner = pool.spawner();
        let finished = Rc::new(RefCell::new(false));
        spawner.spawn_local({
            let finished = finished.clone();
            let semaphore = semaphore.clone();
            async move {
                println!("A");
                semaphore.acquire(5).await;
                println!("B");
                *finished.borrow_mut() = true;
            }
        }).unwrap();
        pool.run_until_stalled();
        assert!(!*finished.borrow());
        mem::drop(g1);
        pool.run_until_stalled();
        assert!(*finished.borrow());
    }

    #[test]
    fn test_interrupt() {
        let semaphore = Rc::new(Semaphore::new(10));
        println!("A");
        let _g1 = block_on(semaphore.acquire(5));
        println!("B");
        let g2 = block_on(semaphore.acquire(5));
        println!("C");
        let mut pool = LocalPool::new();
        let spawner = pool.spawner();
        spawner.spawn_local({
            let semaphore = semaphore.clone();
            async move {
                println!("D");
                semaphore.acquire(5).await;
                println!("E");
            }
        }).unwrap();
        println!("F");
        pool.run_until_stalled();
        println!("G");
        mem::drop(g2);
        println!("H");
        mem::drop(spawner);
        println!("I");
        mem::drop(pool);
        println!("J");
        let _g3 = block_on(semaphore.acquire(5));
        println!("K");
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