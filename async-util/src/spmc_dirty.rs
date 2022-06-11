use std::pin::Pin;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::AcqRel;
use std::sync::Arc;
use std::task::{ready, Context, Poll};

use tokio::sync::Notify;
use tokio_stream::Stream;
use util::weak_vec::WeakVec;

use crate::dirty;
use crate::waker::AtomicWaker;

pub struct Sender {
    generation: Arc<AtomicUsize>,
    wakers: WeakVec<AtomicWaker>,
}

pub struct Receiver {
    old_count: usize,
    count: Arc<AtomicUsize>,
    waker: Arc<AtomicWaker>,
}

impl Sender {
    pub fn new() -> Self { todo!() }
    pub fn subscribe(&mut self) -> Receiver { todo!() }
}

impl Sender {
    pub fn mark(&self) { self.generation.fetch_add(2, AcqRel); }
}

impl Receiver {
    fn try_poll_next(&mut self) -> Poll<Option<()>> {
        let new_count = self.count.load(AcqRel);
        if new_count == 1 {
            Poll::Ready(None)
        } else if new_count == self.old_count {
            Poll::Pending
        } else {
            Poll::Ready(Some(()))
        }
    }
}

impl Stream for Receiver {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        ready!(self.try_poll_next());
        self.waker.register(cx.waker());
        self.try_poll_next()
    }
}

impl Unpin for Receiver {}

impl Drop for Sender {
    fn drop(&mut self) { self.generation.store(1, AcqRel); }
}
