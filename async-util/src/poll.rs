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
use tokio_stream::Stream;
use std::io;
use std::ops::Try;
use crate::poll::PollResult::{Abort, Yield, Noop};

pub enum PollError<Y = (), E = !> {
    Yield(Y),
    Abort(E),
}

#[must_use]
pub enum PollResult<Y = (), E = !> {
    Noop,
    Yield(Y),
    Abort(E),
}

impl<Y, E> Try for PollResult<Y, E> {
    type Ok = ();
    type Error = PollError<Y, E>;

    fn into_result(self) -> Result<Self::Ok, Self::Error> {
        match self {
            Noop => Ok(()),
            Yield(y) => Err(PollError::Yield(y)),
            Abort(e) => Err(PollError::Abort(e)),
        }
    }

    fn from_error(v: Self::Error) -> Self {
        match v {
            PollError::Yield(y) => PollResult::Yield(y),
            PollError::Abort(e) => PollResult::Abort(e),
        }
    }

    fn from_ok(v: ()) -> Self {
        Self::Noop
    }
}

impl<Y, E> PollResult<Y, E> {
    pub fn map<Y2>(self, f: impl FnOnce(Y) -> Y2) -> PollResult<Y2, E> {
        match self {
            Noop => Noop,
            Yield(y) => Yield(f(y)),
            Abort(e) => Abort(e),
        }
    }
}

impl<Y> From<PollError<Y, !>> for PollError<Y, io::Error> {
    fn from(x: PollError<Y, !>) -> Self {
        match x {
            PollError::Yield(y) => PollError::Yield(y),
            PollError::Abort(e) => match e {}
        }
    }
}

pub async fn poll_loop<E, F: for<'a, 'b> FnMut(&'a mut Context<'b>) -> PollResult<(), E>>(mut fun: F) -> Result<!, E> {
    Err(poll_fn(|cx| {
        match fun(cx) {
            Noop => Poll::Pending,
            Yield(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Abort(e) => Poll::Ready(e),
        }
    }).await)
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
    fn guarded<'a, 'b, 'c, E, F: for<'d> FnOnce(&'d mut Context<'d>) -> PollResult<(), E>>(&'a self, cx: &'b mut Context<'c>, fun: F) -> PollResult<(), E> {
        self.0.waker.register(cx.waker());
        if self.0.awake.compare_exchange(1, 0, AcqRel, Acquire).is_ok() {
            match fun(&mut Context::from_waker(&self.0.clone().into())) {
                Noop => Noop,
                Yield(y) => {
                    self.0.wake_by_ref();
                    Yield(y)
                }
                Abort(e) => Abort(e),
            }
        } else {
            Noop
        }
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
    pub fn guarded<E, F: for<'a, 'b> Fn(&'a mut Context<'b>, &T) -> PollResult<(), E>>(&mut self, cx: &mut Context, fun: F) -> PollResult<(), E> {
        let mut lock = self.inner.lock().unwrap();
        while let Some(item) = lock.awake.pop() {
            match fun(cx, &item.value) {
                Noop => (),
                Yield(y) => {
                    item.wake_by_ref();
                    return Yield(y);
                }
                Abort(e) => return Abort(e),
            }
        }
        Noop
    }
}

fn poll_future<F: Future + Unpin>(cx: &mut Context, fut: &mut Option<F>) -> PollResult<F::Output> {
    if let Some(next) = fut {
        if let Poll::Ready(result) = Pin::new(next).poll(cx) {
            *fut = None;
            Yield(result)
        } else {
            Noop
        }
    } else {
        Noop
    }
}

fn poll_take<T>(cx: &mut Context, x: &mut Option<T>) -> PollResult<T> {
    if let Some(x) = x.take() {
        Yield(x)
    } else {
        Noop
    }
}

pub fn poll_next<S: Stream + Unpin>(cx: &mut Context, str: &mut Option<S>) -> PollResult<S::Item> {
    if let Some(str2) = str {
        match Pin::new(str2).poll_next(cx) {
            Poll::Pending => Noop,
            Poll::Ready(None) => {
                *str = None;
                Noop
            }
            Poll::Ready(Some(next)) => {
                Yield(next)
            }
        }
    } else {
        Noop
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