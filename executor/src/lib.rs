#![feature(wake_trait)]
#![feature(test)]

extern crate test;


use std::sync::{Arc, Mutex, Weak};
use std::future::Future;
use std::collections::BinaryHeap;
use std::task::Wake;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::cell::UnsafeCell;
use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use crossbeam::channel::unbounded;
use std::{thread, mem};
use std::mem::size_of;
use test::Bencher;
use std::sync::atomic::Ordering::{Relaxed, AcqRel, SeqCst, Release, Acquire};
use std::cmp::Ordering;

#[derive(Clone)]
struct Executor(Arc<Inner>);

const RUNNING: usize = 0;
const WOKEN: usize = 1;
const PENDING: usize = 2;
const QUEUED: usize = 3;

struct Thread {
    queue: Mutex<BinaryHeap<QueuedTask>>
}

struct Inner {
    threads: Vec<Thread>,
    sender: Sender<Arc<Task>>,
    active: Box<AtomicUsize>,
}

struct QueuedTask {
    priority: usize,
    task: Arc<Task>,
}

struct Task<F: Future<Output=()> + ?Sized = dyn Future<Output=()>> {
    executor: Executor,
    priority: usize,
    flag: AtomicUsize,
    thread: UnsafeCell<usize>,
    inner: UnsafeCell<F>,
}

impl Executor {
    fn new(threads: usize) -> Self {
        assert!(threads <= size_of::<usize>() * 8);
        let (sender, receiver) = unbounded();
        let exec = Executor(Arc::new(Inner {
            threads: (0..threads).map(|i|
                Thread {
                    queue: Mutex::new(BinaryHeap::new())
                }
            ).collect(),
            sender,
            queued: Box::new(AtomicUsize::new(0)),
        }));
        for i in 0..threads {
            let thread = thread::spawn({
                let exec_weak = Arc::downgrade(&exec.0);
                move || {
                    while let Some(exec) = exec_weak.upgrade() {
                        let exec = Executor(exec);
                        exec.run_queued(i);
                        mem::drop(exec);
                        thread::park();
                    }
                }
            }).thread().clone();
        }
        exec
    }
    fn run_all(&self, thread: usize) {
        loop {
            for offset in 0.. {
                let queued = self.0.queued.load(Relaxed);
                if queued < thread {
                    return;
                }
                let pop_thread = (self.0.threads.len() + thread - offset) % self.0.threads.len();
                let mut queue = self.0.threads[thread].queue.lock().unwrap();
                let task = match queue.pop {
                    None => continue,
                    Some(task) => task,
                };
                self.0.queued.fetch_sub(1, Relaxed);
                mem::drop(queue);

            }
        }
    }
    fn spawn(&self, fut: impl Future<Output=()>) {
        unimplemented!()
    }
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        unimplemented!()
    }
}


impl PartialEq for QueuedTask {
    fn eq(&self, other: &Self) -> bool {
        unimplemented!()
    }
}

impl Eq for QueuedTask {}

impl Ord for QueuedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        unimplemented!()
    }
}

impl PartialOrd for QueuedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unimplemented!()
    }
}

unsafe impl Sync for Task {}

unsafe impl Send for Task {}

#[bench]
fn shared(b: &mut Bencher) {
    b.iter(|| {
        let x = Arc::new(AtomicBool::new(false));
        (0..8).map(|_| thread::spawn({
            let x = x.clone();
            move || {
                for _ in 0..1_000_000 {
                    //x.compare_exchange(false, true, AcqRel, Relaxed);
                    //x.fetch_or(true, SeqCst);
                    //#[allow(mutable_transmutes)]
                    //unsafe { *mem::transmute::<&AtomicBool, &mut bool>(& *x) = true; }
                    x.swap(true, Acquire);
                }
            }
        })).collect::<Vec<_>>().into_iter().for_each(|j| j.join().unwrap())
    });
}