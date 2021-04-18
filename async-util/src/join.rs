use std::future::Future;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use crate::waker::AtomicWaker;
use std::lazy::SyncOnceCell;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::Arc;
use pin_project::pin_project;
use std::thread;
use std::cell::UnsafeCell;
use std::panic::{catch_unwind, AssertUnwindSafe, resume_unwind};
use tokio::sync::oneshot;
use crate::futureext::FutureExt;
use tokio::pin;
use tokio::task::yield_now;
use std::any::Any;

pub trait JoinHandle: Future {
    fn abort(self);
}

struct Inner<T> {
    remote: AtomicWaker,
    aborted: AtomicBool,
    handle: AtomicWaker,
    done: AtomicBool,
    result: UnsafeCell<Option<thread::Result<T>>>,
}

#[pin_project]
pub struct Remote<F: Future> {
    inner: Arc<Inner<F::Output>>,
    #[pin]
    fut: F,
}

pub struct RemoteJoinHandle<T> {
    inner: Arc<Inner<T>>,
}

impl<F: Future> Future for Remote<F> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut this = self.project();
        if this.inner.aborted.load(Acquire) {
            return Poll::Ready(());
        } else {
            this.inner.remote.register(cx.waker());
            if this.inner.aborted.load(Acquire) {
                return Poll::Ready(());
            }
        }
        let result = match catch_unwind(AssertUnwindSafe(|| this.fut.as_mut().poll(cx))) {
            Ok(Poll::Ready(result)) => Ok(result),
            Ok(Poll::Pending) => return Poll::Pending,
            Err(error) => Err(error),
        };
        unsafe { this.inner.result.get().write(Some(result)) };
        this.inner.done.store(true, Release);
        this.inner.handle.wake();
        Poll::Ready(())
    }
}

impl<T> Future for RemoteJoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            if !self.inner.done.load(Acquire) {
                self.inner.handle.register(cx.waker());
                if !self.inner.done.load(Acquire) {
                    return Poll::Pending;
                }
            }
            match (*self.inner.result.get()).take().unwrap() {
                Ok(output) => Poll::Ready(output),
                Err(e) => resume_unwind(e),
            }
        }
    }
}

impl<T> JoinHandle for RemoteJoinHandle<T> {
    fn abort(self) {
        self.inner.aborted.store(true, Release);
        self.inner.remote.wake();
    }
}

pub fn remote<F: Future>(fut: F) -> (Remote<F>, RemoteJoinHandle<F::Output>) {
    let inner = Arc::new(Inner {
        remote: AtomicWaker::new(),
        aborted: AtomicBool::new(false),
        handle: AtomicWaker::new(),
        done: AtomicBool::new(false),
        result: UnsafeCell::new(None),
    });
    (Remote { inner: inner.clone(), fut }, RemoteJoinHandle { inner })
}

unsafe impl<F: Future + Send + 'static> Send for Remote<F> where F::Output: Send + 'static {}

unsafe impl<T: Send + 'static> Send for RemoteJoinHandle<T> {}

#[test]
fn join_test() {
    let (sender, receiver) = oneshot::channel();
    let (remote, handle) = receiver.into_remote();
    pin!(remote, handle);
    assert!(handle.as_mut().ready().is_none());
    assert!(remote.as_mut().ready().is_none());
    assert!(handle.as_mut().ready().is_none());
    sender.send(1).unwrap();
    assert!(handle.as_mut().ready().is_none());
    remote.as_mut().ready().unwrap();
    assert_eq!(1, handle.as_mut().ready().unwrap().unwrap());
}

#[tokio::test]
async fn spawn_test() {
    let (sender, receiver) = oneshot::channel();
    let (remote, handle) = receiver.into_remote();
    let join = tokio::spawn(remote);
    pin!(handle, join);
    sender.send(1).unwrap();
    assert!(handle.as_mut().ready().is_none());
    assert!(join.as_mut().ready().is_none());
    yield_now().await;
    assert_eq!(1, handle.as_mut().ready().unwrap().unwrap());
    join.as_mut().ready().unwrap().unwrap();
}


#[tokio::test]
#[should_panic(expected = "PANIC 42")]
async fn panic_test() {
    let (remote, handle) = async move {
        panic!("PANIC {}", 42);
        #[allow(unreachable_code)]
        ()
    }.into_remote();
    let join = tokio::spawn(remote);
    pin!(handle, join);
    assert!(handle.as_mut().ready().is_none());
    assert!(join.as_mut().ready().is_none());
    yield_now().await;
    handle.as_mut().ready().unwrap();
}