use std::cmp::Ordering;
use std::ops::Add;
use std::pin::Pin;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::mpsc::RecvError;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::sync::Barrier;
use tokio::task::yield_now;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};
use tokio_stream::{Stream, StreamExt};

use crate::futureext::FutureExt;
use crate::waker::AtomicWaker;

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
        match self
            .0
            .state
            .compare_exchange(STATE_DIRTY, STATE_CLEAN, Relaxed, Relaxed)
        {
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

#[tokio::test]
async fn test_simple() {
    let (sender, mut receiver) = channel();
    assert!(receiver.next().ready().is_none());
    sender.mark();
    receiver.next().ready().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multithread() {
    let (sender, receiver) = channel();
    let barrier1 = Arc::new(Barrier::new(2));
    let barrier2 = barrier1.clone();
    let h1 = tokio::spawn(async move {
        barrier1.wait().await;
        for x in 0..100000 {
            if x % 1000 == 0 {
                yield_now().await;
            }
            sender.mark();
        }
    });
    let h2 = tokio::spawn(async move {
        barrier2.wait().await;
        receiver.map(|x| 1).fold(0usize, usize::add).await
    });
    h1.await.unwrap();
    let count = h2.await.unwrap();
    dbg!(count);
    assert!(count > 10);
}
