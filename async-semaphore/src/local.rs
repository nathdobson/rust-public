use std::mem;
use std::task::{Waker, Poll};
use std::cell::RefCell;
use std::future::Future;
use crate::waker::{yield_once, clone_waker};
use crate::queue::Queue;
use crate::WouldBlock;

struct Waiter {
    waker: Waker,
    amount: u64,
}

pub struct Semaphore { inner: RefCell<SemaphoreInner> }

struct SemaphoreInner {
    available: u64,
    waiting: Queue<Waiter>,
}

pub struct SemaphoreGuard<'a> {
    semaphore: &'a Semaphore,
    amount: u64,
}

impl SemaphoreInner {
    fn new(initial: u64) -> Self {
        SemaphoreInner {
            available: initial,
            waiting: Queue::new(),
        }
    }
}

impl SemaphoreGuard<'_> {
    pub fn forget(self) {
        mem::forget(self)
    }
}

impl Semaphore {
    pub fn new(capacity: u64) -> Semaphore {
        Semaphore {
            inner: RefCell::new(SemaphoreInner::new(capacity))
        }
    }

    pub fn try_acquire(&self, amount: u64) -> Result<SemaphoreGuard<'_>, WouldBlock> {
        let mut this = self.inner.borrow_mut();
        if amount <= this.available {
            this.available -= amount;
            Ok(SemaphoreGuard { semaphore: self, amount })
        } else {
            Err(WouldBlock)
        }
    }

    pub async fn acquire(&self, amount: u64) -> SemaphoreGuard<'_> {
        let mut this = self.inner.borrow_mut();
        if amount <= this.available {
            this.available -= amount;
            return SemaphoreGuard { semaphore: self, amount };
        }

        let id = this.waiting.push_back(Waiter {
            waker: clone_waker().await,
            amount,
        });
        loop {
            mem::drop(this);
            yield_once(|| {
                if id < self.inner.borrow_mut().waiting.front_key() {
                    self.release(amount);
                }
            }).await;
            this = self.inner.borrow_mut();
            if id < this.waiting.front_key() {
                return SemaphoreGuard { semaphore: self, amount };
            } else {
                let call = this.waiting.get_mut(id).unwrap();
                call.waker = clone_waker().await;
            }
        }
    }
    pub fn release(&self, amount: u64) {
        let mut this = self.inner.borrow_mut();
        this.available += amount;
        while let Some(front) = this.waiting.front() {
            if front.amount > this.available {
                break;
            }
            let waker = this.waiting.pop_front().unwrap().waker;
            this.available -= amount;
            mem::drop(this);
            waker.wake();
            this = self.inner.borrow_mut();
        }
        mem::drop(this);
    }
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        self.semaphore.release(self.amount);
    }
}

#[cfg(test)]
mod test {
    use rand_xorshift::XorShiftRng;
    use std::{mem, thread};
    use futures::executor::{LocalPool, block_on};
    use std::rc::Rc;
    use futures::task::{LocalSpawnExt, SpawnExt, LocalFutureObj, FutureObj};
    use rand::{SeedableRng, Rng, thread_rng};
    use std::time::{Duration, Instant};
    use async_std::future::{timeout, TimeoutError};
    use async_std::task::sleep;
    use std::cell::RefCell;
    use crate::local::{Semaphore, SemaphoreGuard};
    use crate::WouldBlock;
    use defer::defer;
    use futures::future::poll_fn;
    use futures::{Future, StreamExt};
    use futures::future::pending;
    use std::task::Poll;
    use futures::poll;
    use std::process::abort;
    use futures::stream::FuturesUnordered;

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
}