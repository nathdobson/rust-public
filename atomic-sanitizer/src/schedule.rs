use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::channel;
use std::sync::{Arc, Barrier};
use std::thread;
use std::thread::JoinHandle;

use futures::executor::ThreadPool;

// lazy_static! {
//     static ref NEXT_THREAD_ID: AtomicUsize = AtomicUsize::new(0);
// }

thread_local! {
    static THREAD_DATA: RefCell<Option<ThreadData>> = RefCell::new(None);
}

struct ThreadData {
    pool: Arc<SeqThreadPool>,
    id: usize,
}

struct SeqThreadPool {
    pool: ThreadPool,
}

impl SeqThreadPool {
    pub fn new(threads: usize) -> Arc<Self> {
        let pool = ThreadPool::builder().pool_size(threads).create().unwrap();
        let seq_pool = Arc::new(SeqThreadPool { pool });
        for thread in 0..threads {
            let (send, receive) = channel();
            let barrier = barrier.clone();
            let seq_pool2 = seq_pool.clone();
            seq_pool.pool.spawn_ok(async move {
                THREAD_DATA.with(|thread_data| {
                    *thread_data.borrow_mut() = Some(ThreadData {
                        pool: seq_pool2,
                        id: 0,
                    })
                });
                send.send(()).unwrap();
                barrier.wait();
            });
            receive.recv().unwrap();
        }
        seq_pool
    }
    // pub fn run(count: usize, f: impl Fn(usize) + Send + Sync + 'static) {
    //     let f = Arc::new(f);
    //     (0..count).map(|i| {
    //         let f = f.clone();
    //         thread::executor(move || {
    //             THREAD_ID.with(|thread_id| {
    //                 thread_id.set(i);
    //             });
    //             f(i)
    //         })
    //     }).collect::<Vec<_>>().into_iter().for_each(|join| join.join().unwrap());
    // }
}
