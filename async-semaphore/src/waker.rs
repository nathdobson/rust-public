use crate::atomic::{AtomicPacker, Atomic};
use std::sync::atomic::AtomicU128;
use std::task::{Waker, Poll, Context};
use std::mem;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering::AcqRel;

pub struct WakerPacker;

impl AtomicPacker for WakerPacker {
    type Impl = AtomicU128;
    type Value = Option<Waker>;

    unsafe fn decode(x: u128) -> Option<Waker> {
        match x {
            0 => None,
            _ => Some(mem::transmute(x)),
        }
    }
    unsafe fn encode(x: Option<Waker>) -> u128 {
        match x {
            None => 0,
            Some(x) => mem::transmute(x),
        }
    }
}

pub struct AtomicWaker(Atomic<WakerPacker>);

impl AtomicWaker {
    pub fn new() -> impl Future<Output=AtomicWaker> {
        struct New;
        impl Future for New {
            type Output = AtomicWaker;
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(AtomicWaker(Atomic::new(Some(cx.waker().clone()))))
            }
        }
        New
    }
    pub fn wait<'b>(&'b self, on_cancel: impl FnOnce(bool) + 'b) -> impl Future + 'b {
        struct Wait<'a, F: FnOnce(bool)> {
            waker: &'a AtomicWaker,
            entered: bool,
            on_cancel: Option<F>,
        }
        impl<'a, F: FnOnce(bool)> Future for Wait<'a, F> {
            type Output = ();
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                unsafe {
                    let this = self.get_unchecked_mut();
                    if !this.entered {
                        this.entered = true;
                        return Poll::Pending;
                    }
                    let old = this.waker.0.swap(Some(cx.waker().clone()), AcqRel);
                    if old.is_none() {
                        this.on_cancel = None;
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                }
            }
        }
        impl<'a, F: FnOnce(bool)> Drop for Wait<'a, F> {
            fn drop(&mut self) {
                if let Some(on_cancel) = self.on_cancel.take().take() {
                    let old = self.waker.0.swap(None, AcqRel);
                    on_cancel(old.is_none());
                }
            }
        }
        Wait {
            waker: self,
            entered: false,
            on_cancel: Some(on_cancel),
        }
    }

    #[must_use]
    pub fn notify(&self) -> bool {
        if let Some(waker) = self.0.swap(None, AcqRel) {
            waker.wake();
            true
        } else {
            false
        }
    }
}