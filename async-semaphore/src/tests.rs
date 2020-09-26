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
use crate::{ReleaseGuard, AcquireRelease, Semaphore};

macro_rules! run_all {
    ($fun: ident) => {
        mod $fun{
            #[test] fn shared_dwcas(){ super::$fun::<crate::shared_dwcas::SemaphoreImpl>(); }
            #[test] fn shared_swcas(){ super::$fun::<crate::shared_swcas::SemaphoreImpl>(); }
            #[test] fn shared_mutex(){ super::$fun::<crate::shared_mutex::SemaphoreImpl>(); }
        }
    }
}

run_all!(test_simple);
fn test_simple<T: AcquireRelease+'static>() {
    println!("A");
    let semaphore = Rc::new(Semaphore::<T>::new(10));
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

struct CheckedSemaphore<T: AcquireRelease> {
    capacity: usize,
    semaphore: Semaphore<T>,
    counter: Mutex<usize>,
}

impl<T: AcquireRelease> CheckedSemaphore<T> {
    fn new(capacity: usize) -> Self {
        CheckedSemaphore {
            capacity,
            semaphore: Semaphore::new(capacity),
            counter: Mutex::new(0),
        }
    }
    async fn acquire(&self, amount: usize) -> ReleaseGuard<T> {
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

run_all!(test_multicore);
fn test_multicore<T: AcquireRelease + Send + 'static + Sync>() where T::Acq:Send {
    let capacity = 100;
    let semaphore = Arc::new(CheckedSemaphore::<T>::new(capacity));
    let pool = ThreadPool::builder().pool_size(10).create().unwrap();
    (0..10).map(|_thread|
        pool.spawn_with_handle({
            let semaphore = semaphore.clone();
            async move {
                //let indent = " ".repeat(thread * 10);
                let mut owned = 0;
                for _i in 0..100 {
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
