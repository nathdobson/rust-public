use std::sync::Arc;
use crate::waker::AtomicWaker;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::atomic::Ordering::Relaxed;
use futures_util::stream::Stream;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::sync::mpsc::RecvError;
use std::cmp::Ordering;
use futures::executor::{ThreadPool, block_on};
use futures::task::SpawnExt;
use async_std::stream::StreamExt;

const STATE_CLEAN: usize = 0;
const STATE_DIRTY: usize = 1;
const STATE_CLOSED: usize = 2;

struct Inner {
    waker: AtomicWaker,
    state: AtomicUsize,
    refcount: AtomicUsize,
}

pub struct Sender(Arc<Inner>);

pub struct Receiver(Arc<Inner>);

impl Sender {
    pub fn mark(&self) {
        self.0.state.store(STATE_DIRTY, Relaxed);
        self.0.waker.wake();
    }
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        self.0.refcount.fetch_add(1, Relaxed);
        Sender(self.0.clone())
    }
}

impl Drop for Sender {
    fn drop(&mut self) {
        if self.0.refcount.fetch_sub(1, Relaxed) == 1 {
            self.0.state.store(STATE_CLOSED, Relaxed);
            self.0.waker.wake();
        }
    }
}

impl Receiver {
    fn try_poll_next(&mut self) -> Poll<Option<()>> {
        match self.0.state.compare_exchange(STATE_DIRTY, STATE_CLEAN, Relaxed, Relaxed) {
            Ok(STATE_DIRTY) => Poll::Ready(Some(())),
            Err(STATE_CLEAN) => Poll::Pending,
            Err(STATE_CLOSED) => Poll::Ready(None),
            _ => panic!(),
        }
    }
}

impl Stream for Receiver {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.try_poll_next() {
            Poll::Pending => {
                self.0.waker.register(cx.waker());
                self.try_poll_next()
            }
            Poll::Ready(x) => Poll::Ready(x),
        }
    }
}

impl Unpin for Receiver {}

pub fn channel() -> (Sender, Receiver) {
    let inner = Arc::new(Inner {
        waker: AtomicWaker::new(),
        state: AtomicUsize::new(STATE_CLEAN),
        refcount: AtomicUsize::new(1),
    });
    (Sender(inner.clone()), Receiver(inner))
}

#[test]
fn test() {
    let pool = ThreadPool::new().unwrap();

    let (sender, receiver) = channel();
    let h1 = pool.spawn_with_handle(async move {
        for x in 0..100000 {
            sender.mark();
        }
    }).unwrap();
    let h2 = pool.spawn_with_handle(async move {
        receiver.count().await
    }).unwrap();
    block_on(h1);
    assert!(block_on(h2) > 1000);
}