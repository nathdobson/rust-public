use std::sync::Arc;
use futures::executor::{ThreadPool, block_on};
use rand::{thread_rng, Rng};
use std::time::Duration;
use futures::task::SpawnExt;
use crate::{shared_mutex, Releaser};
use test::Bencher;

use itertools::Itertools;
use std::{thread, mem};

//use crate::shared_mutex::Semaphore;
use std::sync::atomic::AtomicU64;
use crate::shared_dwcas::Semaphore;

#[inline(never)]
fn noopt(_: usize) {}

#[bench]
fn run_shared(b: &mut Bencher) where {
    let capacity = 100;
    let threads = 4;
    let futures = 300;
    let iterations = 500;
    let pool = ThreadPool::builder().pool_size(threads).create().unwrap();

    b.iter(|| {
        let semaphore = Arc::new(Semaphore::new(capacity));
        (0..futures).map(|_future| {
            pool.spawn_with_handle({
                let semaphore = semaphore.clone();
                async move {
                    let mut owned = 0;
                    for _ in 0..iterations {
                        if owned == 0 {
                            owned = thread_rng().gen_range(1, capacity + 1);
                            //println!("Acquiring {:?} {:?} {:?}", future, owned, semaphore);
                            semaphore.acquire(owned).await.forget();
                            //println!("Acquired  {:?} {:?} {:?}", future, owned, semaphore);
                            for i in 0..2000 {
                                noopt(i);
                            }
                        } else {
                            let mut rng = thread_rng();
                            let r = if rng.gen_bool(0.5) {
                                owned
                            } else {
                                rng.gen_range(1, owned + 1)
                            };
                            owned -= r;
                            //println!("Releasing {:?} {:?} {:?}", future, r, semaphore);
                            semaphore.release(r);
                            //println!("Released  {:?} {:?} {:?}", future, r, semaphore);
                        }
                    }
                    //println!("Finishing {:?} {:?} {:?}", future, owned, semaphore);
                    semaphore.release(owned);
                    //println!("Finished  {:?} {:?} {:?}", future, owned, semaphore);
                }
            }).unwrap()
        })
            .collect::<Vec<_>>().into_iter().for_each(|x| block_on(x));
    })
}
