use futures::pending;
use std::mem;
use std::task::{Waker, Poll};
use std::cell::RefCell;
use crate::queue::Queue;
use std::future::Future;
use futures::future::poll_fn;
use crate::{future_waker, LockError};

struct Waiter {
    waker: Waker,
    amount: u64,
}

pub struct Semaphore { inner: RefCell<SemaphoreInner> }

struct SemaphoreInner {
    capacity: u64,
    available: u64,
    waiting: Queue<Waiter>,
}

pub struct SemaphoreGuard<'a> {
    semaphore: &'a Semaphore,
    amount: u64,
}

impl SemaphoreInner {
    fn new(capacity: u64) -> Self {
        SemaphoreInner {
            capacity,
            available: capacity,
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
    pub async fn acquire(&self, amount: u64) -> Result<SemaphoreGuard<'_>, LockError> {
        let mut this = self.inner.borrow_mut();
        if amount > this.capacity {
            return Err(LockError::WouldDeadlock);
        }
        if amount <= this.available {
            this.available -= amount;
            return Ok(SemaphoreGuard { semaphore: self, amount });
        }

        let id = this.waiting.push_back(Waiter {
            waker: future_waker().await,
            amount,
        });
        loop {
            mem::drop(this);
            let cancel_guard = defer::defer(|| {
                if id < self.inner.borrow_mut().waiting.front_key() {
                    self.release(amount);
                }
            });
            pending!();
            mem::forget(cancel_guard);
            this = self.inner.borrow_mut();
            if id < this.waiting.front_key() {
                return Ok(SemaphoreGuard { semaphore: self, amount });
            } else {
                let call = this.waiting.get_mut(id).unwrap();
                call.waker = future_waker().await;
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
    use std::mem;
    use futures::executor::{LocalPool, block_on};
    use std::rc::Rc;
    use futures::task::LocalSpawnExt;
    use rand::{SeedableRng, Rng};
    use std::time::Duration;
    use async_std::future::{timeout, TimeoutError};
    use async_std::task::sleep;
    use std::cell::RefCell;
    use crate::local::{Semaphore, SemaphoreGuard};
    use crate::LockError;

    #[test]
    fn test() {
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
                        Ok(Ok(guard)) => {
                            println!("{}B", indent);
                            let time = rng.gen_range(0, 10);
                            sleep(Duration::from_millis(time)).await;
                            println!("{}C", indent);
                            mem::drop(guard);
                            println!("{}D", indent);
                        }
                        Ok(Err(_)) => panic!("Shouldn't error"),
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
        block_on(semaphore.acquire(0)).unwrap();
        assert!(block_on(semaphore.acquire(1)).contains_err(&LockError::WouldDeadlock));
    }

    #[test]
    fn test_shared() {
        let semaphore = Rc::new(Semaphore::new(10));
        let g1 = block_on(semaphore.acquire(5)).unwrap();
        let g2 = block_on(semaphore.acquire(5)).unwrap();
        let mut pool = LocalPool::new();
        let spawner = pool.spawner();
        let finished = Rc::new(RefCell::new(false));
        spawner.spawn_local({
            let finished = finished.clone();
            let semaphore = semaphore.clone();
            async move {
                println!("A");
                semaphore.acquire(5).await.unwrap();
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
        let g1 = block_on(semaphore.acquire(5)).unwrap();
        let g2 = block_on(semaphore.acquire(5)).unwrap();
        let mut pool = LocalPool::new();
        let spawner = pool.spawner();
        spawner.spawn_local({
            let semaphore = semaphore.clone();
            async move {
                semaphore.acquire(5).await.unwrap();
            }
        }).unwrap();
        pool.run_until_stalled();
        mem::drop(g2);
        mem::drop(spawner);
        mem::drop(pool);
        let g3 = block_on(semaphore.acquire(5)).unwrap();
    }
}