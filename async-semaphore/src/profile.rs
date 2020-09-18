use std::sync::{Arc};
use std::sync::atomic::AtomicUsize;
use std::future::Future;
use std::task::{Waker, Wake};
use std::sync::atomic::Ordering::Relaxed;
use futures::task::{Context, Poll};
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct Profile(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    polls: AtomicUsize,
    wakes: AtomicUsize,
}

impl Profile {
    pub fn new() -> Self {
        Profile(Arc::new(Inner {
            polls: AtomicUsize::new(0),
            wakes: AtomicUsize::new(0),
        }))
    }
    pub fn wrap<T>(&self, inner: impl Future<Output=T>) -> impl Future<Output=T> {
        struct WrappedWaker {
            profile: Profile,
            inner: Waker,
        }
        impl Wake for WrappedWaker {
            fn wake(self: Arc<Self>) {
                self.profile.0.wakes.fetch_add(1, Relaxed);
                self.inner.wake_by_ref();
            }
        }
        struct Wrap<F> {
            profile: Profile,
            inner: F,
        }
        impl<F: Future> Future for Wrap<F> {
            type Output = F::Output;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                unsafe {
                    let this = self.get_unchecked_mut();
                    this.profile.0.polls.fetch_add(1, Relaxed);
                    Pin::new_unchecked(&mut this.inner).poll(
                        &mut Context::from_waker(
                            &Waker::from(
                                Arc::new(
                                    WrappedWaker {
                                        profile: this.profile.clone(),
                                        inner: cx.waker().clone(),
                                    }))))
                }
            }
        }
        Wrap {
            profile: self.clone(),
            inner,
        }
    }
}