#![feature(never_type)]
#![feature(trivial_bounds)]
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use core::mem;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use bit_set::BitSet;
use futures::FutureExt;
use slab::Slab;
use tokio::select;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::RecvError;

pub enum JoinError {
    Canceled,
    Panic(Box<dyn 'static + Send + Any>),
}

impl Debug for JoinError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinError::Canceled => write!(f, "JoinError::Canceled"),
            JoinError::Panic(_) => write!(f, "JoinError::Panic(...)"),
        }
    }
}

pub type Result<T> = std::result::Result<T, JoinError>;

struct LocalPool {
    tasks: Slab<Cell<Option<Pin<Box<dyn 'static + Future<Output = ()>>>>>>,
    queue: VecDeque<usize>,
    woken: bit_set::BitSet,
    canceled: bit_set::BitSet,
    waker: Option<Waker>,
}

struct RunLocal;

#[derive(Debug)]
pub struct JoinHandle<T> {
    task_id: usize,
    receiver: oneshot::Receiver<std::thread::Result<T>>,
}

static RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| RawWaker::new(data, &RAW_WAKER_VTABLE),
    |x| wake(x as usize),
    |x| wake(x as usize),
    |x| (),
);

static ONE_THREAD: AtomicBool = AtomicBool::new(false);

thread_local! {
    static LOCAL_POOL: RefCell<Option<LocalPool>> = RefCell::new(Some(LocalPool::new()));
}

impl LocalPool {
    fn new() -> Self {
        assert!(!ONE_THREAD.swap(true, Ordering::SeqCst));
        LocalPool {
            tasks: Slab::new(),
            queue: VecDeque::new(),
            woken: BitSet::new(),
            canceled: BitSet::new(),
            waker: None,
        }
    }
}

impl<T> JoinHandle<T> {
    pub fn cancel(&self) {
        LOCAL_POOL.with(|local_pool_cell| {
            let mut local_pool = local_pool_cell.borrow_mut();
            if let Some(local_pool) = &mut *local_pool {
                if local_pool.canceled.insert(self.task_id) {
                    local_pool.queue.push_back(self.task_id);
                }
                local_pool.waker.take().map(|x| x.wake());
            }
        })
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.receiver).poll(cx).map(|x| Ok(x??))
    }
}

impl<T> Unpin for JoinHandle<T> {}

impl Unpin for RunLocal {}

impl Future for RunLocal {
    type Output = !;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<!> {
        LOCAL_POOL.with(|local_pool_cell| {
            let mut local_pool = local_pool_cell.borrow_mut();
            while let Some(task_id) = local_pool.as_mut().unwrap().queue.pop_front() {
                if local_pool.as_mut().unwrap().canceled.remove(task_id) {
                    let task = local_pool.as_mut().unwrap().tasks.remove(task_id);
                    mem::drop(local_pool);
                    mem::drop(task);
                    local_pool = local_pool_cell.borrow_mut();
                } else if local_pool.as_mut().unwrap().woken.remove(task_id) {
                    let mut task = local_pool.as_mut().unwrap().tasks[task_id].take().unwrap();
                    mem::drop(local_pool);
                    let raw_waker = RawWaker::new(task_id as *const (), &RAW_WAKER_VTABLE);
                    let waker = unsafe { Waker::from_raw(raw_waker) };
                    let mut context = Context::from_waker(&waker);
                    if let Poll::Ready(()) = task.as_mut().poll(&mut context) {
                        mem::drop(task);
                        local_pool = local_pool_cell.borrow_mut();
                        local_pool.as_mut().unwrap().tasks.remove(task_id);
                    } else {
                        local_pool = local_pool_cell.borrow_mut();
                        local_pool.as_mut().unwrap().tasks[task_id].set(Some(task));
                    }
                } else {
                    unreachable!();
                }
            }
            local_pool.as_mut().unwrap().waker = Some(cx.waker().clone());
            Poll::Pending
        })
    }
}

fn wake(id: usize) {
    LOCAL_POOL.with(|local_pool_cell| {
        let mut local_pool = local_pool_cell.borrow_mut();
        if let Some(local_pool) = &mut *local_pool {
            if local_pool.woken.insert(id) {
                local_pool.queue.push_back(id);
            }
            local_pool.waker.take().map(|x| x.wake());
        }
    })
}

impl From<RecvError> for JoinError {
    fn from(x: RecvError) -> Self { JoinError::Canceled }
}

impl From<Box<dyn 'static + Send + Any>> for JoinError {
    fn from(x: Box<dyn 'static + Send + Any>) -> Self { JoinError::Panic(x) }
}

pub async fn run() -> ! { RunLocal.await }

pub async fn run_until<F>(f: F) -> F::Output
where
    F: Future,
{
    let result = select! {
        x = RunLocal => x,
        x = f => x,
    };
    LOCAL_POOL.with(|local_pool_cell| {
        let mut local_pool = local_pool_cell.borrow_mut();
        let local_pool_value = local_pool.take();
        mem::drop(local_pool);
        mem::drop(local_pool_value);
    });
    result
}

pub fn spawn<F: Future + 'static>(fut: F) -> JoinHandle<F::Output>
where
    F::Output: 'static,
{
    LOCAL_POOL.with(|local_pool_cell| {
        let mut local_pool = local_pool_cell.borrow_mut();
        let (tx, rx) = oneshot::channel();
        if let Some(local_pool) = &mut *local_pool {
            let task_id = local_pool.tasks.insert(Cell::new(None));
            let runner = async move {
                tx.send(AssertUnwindSafe(fut).catch_unwind().await).ok();
            };
            local_pool.tasks[task_id].set(Some(
                Box::pin(runner) as Pin<Box<dyn 'static + Future<Output = ()>>>
            ));
            local_pool.woken.insert(task_id);
            local_pool.queue.push_back(task_id);
            local_pool.waker.take().map(|x| x.wake());
            JoinHandle {
                task_id,
                receiver: rx,
            }
        } else {
            mem::drop(local_pool);
            mem::drop(fut);
            mem::drop(tx);
            JoinHandle {
                task_id: 0,
                receiver: rx,
            }
        }
    })
}
