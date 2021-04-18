use std::future::Future;
use std::future::poll_fn;
use std::mem;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tokio::task::yield_now;
use tokio::time::sleep;

use util::dirty::Dirty;
use util::mutrc::{MutRc, ReadGuard, WriteGuard};
use util::time::SerialInstant;

use crate::{dirty, priority};
use crate::dirty::Sender;
use crate::timer::poll_elapse;
use crate::waker::AtomicWaker;
use crate::spawn::Spawn;

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

#[tokio::test]
async fn test() {
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
    spawner.with_priority(0).spawn(state_runner);
    let delta = Duration::from_millis(100);
    let epsilon = Duration::from_millis(10);
    spawner.with_priority(1).spawn(async move {
        state.write().next = Some(SerialInstant::now() + delta * 2);
        yield_now().await;
        assert!(state.read().next.is_some());
        state.write().next = Some(SerialInstant::now() + delta);
        yield_now().await;
        assert!(state.read().next.is_some());
        sleep(delta + epsilon).await;
        assert!(state.read().next.is_none());
        assert_eq!(state.read().count, 1);
        mem::drop(state);
    });
    mem::drop(spawner);
    runner.await;
}