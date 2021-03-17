use util::mutrc::{MutRc, WriteGuard, ReadGuard};
use util::dirty::Dirty;
use crate::dirty::Sender;
use std::future::Future;
use crate::{dirty, priority};
use futures::StreamExt;
use std::task::{Poll, Context};
use crate::waker::AtomicWaker;
use std::sync::Arc;
use futures::future::poll_fn;
use util::time::SerialInstant;
use crate::timer::poll_elapse;
use std::time::{Duration, Instant};
use futures::executor::{LocalPool, block_on};
use futures::task::SpawnExt;
use std::mem;
use async_std::task;

pub struct MutFuture<T: 'static> {
    inner: MutRc<T>,
    waker: Arc<AtomicWaker>,
}

impl<T: 'static> MutFuture<T> {
    pub fn new(x: T, mut poller: impl FnMut(&mut T, &mut Context) -> Poll<()> + 'static) -> (Self, impl Future<Output=()>) {
        let inner = MutRc::new(x);
        let waker = Arc::new(AtomicWaker::new());
        let waker2 = waker.clone();
        let weak = MutRc::downgrade(&inner);
        (MutFuture { inner, waker }, poll_fn(move |cx| {
            waker2.register(cx.waker());
            if let Some(mut strong) = weak.upgrade() {
                poller(&mut *strong.write(), cx)
            } else {
                Poll::Ready(())
            }
        }))
    }
    pub fn write(&mut self) -> WriteGuard<T> {
        self.waker.wake();
        self.inner.write()
    }
    pub fn read(&self) -> ReadGuard<T> {
        self.inner.read()
    }
}

impl<T: 'static> Drop for MutFuture<T> {
    fn drop(&mut self) {
        self.waker.wake();
    }
}

#[test]
fn test() {
    #[derive(Debug)]
    struct State {
        count: usize,
        next: Option<SerialInstant>,
    }
    impl State {
        fn poll_state(&mut self, cx: &mut Context) -> Poll<()> {
            if let Some(next) = self.next {
                if poll_elapse(cx, next).is_ready() {
                    self.count += 1;
                    self.next = None;
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
            }
            Poll::Pending
        }
    }
    let (spawner, runner) = priority::channel();
    let state = State { count: 0, next: None };
    let (mut state, state_runner) = MutFuture::new(state, State::poll_state);
    spawner.spawn(0, state_runner);
    let delta = Duration::from_millis(100);
    let epsilon = Duration::from_millis(10);
    spawner.spawn(1, async move {
        state.write().next = Some(SerialInstant::now() + delta * 2);
        task::yield_now().await;
        assert!(state.read().next.is_some());
        state.write().next = Some(SerialInstant::now() + delta);
        task::yield_now().await;
        assert!(state.read().next.is_some());
        task::sleep(delta + epsilon).await;
        assert!(state.read().next.is_none());
        assert_eq!(state.read().count, 1);
        mem::drop(state);
    });
    mem::drop(spawner);
    block_on(runner);
}