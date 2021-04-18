use std::task::{Context, Wake, Poll, Waker};
use std::sync::{Arc, Mutex, Weak};
use std::sync::atomic::{AtomicUsize, Ordering, AtomicBool};
use std::future::{poll_fn, Future};
use std::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release};
use crate::waker::AtomicWaker;
use std::hash::Hash;
use std::collections::{HashMap, HashSet};
use tokio::sync::oneshot;
use crate::fused::Fused;
use std::pin::Pin;
use crate::futureext::FutureExt;

pub struct Yield;

pub type PollResult = Result<(), Yield>;

trait PollFn = for<'a> FnOnce(&'a mut Context<'a>) -> PollResult;

async fn run_loop<F: for<'a, 'b> FnMut(&'a mut Context<'b>) -> PollResult>(mut fun: F) -> ! {
    poll_fn(|cx| {
        if let Err(Yield) = fun(cx) {
            cx.waker().wake_by_ref();
        }
        Poll::Pending
    }).await
}

struct WakeGuardInner {
    awake: AtomicUsize,
    waker: AtomicWaker,
}

pub struct WakeGuard(Arc<WakeGuardInner>);

impl Wake for WakeGuardInner {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref()
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.awake.store(1, Release);
        self.waker.wake();
    }
}

impl WakeGuard {
    pub fn new() -> Self {
        WakeGuard(Arc::new(WakeGuardInner { awake: AtomicUsize::new(1), waker: AtomicWaker::new() }))
    }
    fn guarded<'a, 'b, 'c, F: for<'d> FnOnce(&'d mut Context<'d>) -> PollResult>(&'a self, cx: &'b mut Context<'c>, fun: F) -> PollResult {
        self.0.waker.register(cx.waker());
        if self.0.awake.compare_exchange(1, 0, AcqRel, Acquire).is_ok() {
            if let Err(Yield) = fun(&mut Context::from_waker(&self.0.clone().into())) {
                self.0.wake_by_ref();
                return Err(Yield);
            }
        }
        Ok(())
    }
}

struct WakeGuardSetInner<T> {
    awake: Vec<Arc<WakeGuardSetItem<T>>>,
    waker: Option<Waker>,
}

struct WakeGuardSetItem<T> {
    inner: Weak<Mutex<WakeGuardSetInner<T>>>,
    awake: AtomicBool,
    value: T,
}

pub struct WakeGuardSet<T: Hash + Eq> {
    inner: Arc<Mutex<WakeGuardSetInner<T>>>,
}

impl<T> Wake for WakeGuardSetItem<T> {
    fn wake(self: Arc<Self>) {
        if let Some(inner) = self.inner.upgrade() {
            let mut lock = inner.lock().unwrap();
            self.awake.store(true, Relaxed);
            lock.waker.take().map(|w| w.wake());
        }
    }
}

impl<T: Hash + Eq> WakeGuardSet<T> {
    pub fn new() -> Self {
        WakeGuardSet { inner: Arc::new(Mutex::new(WakeGuardSetInner { awake: Vec::new(), waker: None })) }
    }
    pub fn push(&mut self, value: T) {
        self.inner.lock().unwrap().awake
            .push(Arc::new(WakeGuardSetItem { inner: Arc::downgrade(&self.inner), awake: AtomicBool::new(true), value }));
    }
    pub fn guarded<F: for<'a, 'b> Fn(&'a mut Context<'b>, &T) -> PollResult>(&mut self, cx: &mut Context, fun: F) -> PollResult {
        let mut lock = self.inner.lock().unwrap();
        while let Some(item) = lock.awake.pop() {
            if let Err(Yield) = fun(cx, &item.value) {
                item.wake();
                return Err(Yield);
            }
        }
        Ok(())
    }
}

fn poll_future<F: Future + Unpin, F2: FnOnce(F::Output)>(cx: &mut Context, fut: &mut Option<F>, callback: F2) -> PollResult {
    if let Some(next) = fut {
        if let Poll::Ready(result) = Pin::new(next).poll(cx) {
            callback(result);
            *fut = None;
            Err(Yield)
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

fn poll_take<T, F: FnOnce(T)>(cx: &mut Context, x: &mut Option<T>, callback: F) -> PollResult {
    if let Some(x) = x.take() {
        callback(x);
        Err(Yield)
    } else {
        Ok(())
    }
}

#[tokio::test]
async fn test_wake_guard() {
    let g1 = WakeGuard::new();
    let g2 = WakeGuard::new();
    let (tx, rx) = oneshot::channel();
    let mut rx = Some(rx);
    let mut tx = Some(tx);
    let seq = Mutex::new(vec![]);
    assert!(run_loop(|cx| {
        println!("both");
        seq.lock().unwrap().push(1);
        g1.guarded(cx, |cx| {
            println!("first");
            seq.lock().unwrap().push(2);
            poll_future(cx, &mut rx, |x| {
                assert_eq!(x, Ok(1));
                println!("receive");
                seq.lock().unwrap().push(3);
            })
        })?;
        g2.guarded(cx, |cx| {
            println!("second");
            seq.lock().unwrap().push(4);
            poll_take(cx, &mut tx, |tx| {
                println!("send");
                tx.send(1).unwrap();
                seq.lock().unwrap().push(5);
            })
        })?;
        Ok(())
    }).ready().is_none());
    assert_eq!(*seq.lock().unwrap(), vec![1, 2, 4, 5, 1, 2, 3, 1, 2, 4]);
}