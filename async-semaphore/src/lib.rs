#![allow(unused_imports)]
#![feature(integer_atomics)]
#![feature(is_sorted)]
#![feature(test)]
#![feature(result_contains_err)]
extern crate test;
extern crate futures;

pub mod atomic;
//pub mod concurrent_queue;
pub mod queue;
pub mod stack;
//pub mod mpsc_queue;
pub mod mpmc;
//pub mod local;

use std::fmt::Display;
use std::error::Error;
use std::future::Future;
use std::task::{Waker, Poll};
use futures::future::poll_fn;
use futures::task::Context;
use std::pin::Pin;
use futures::pending;
use std::mem;
use defer::defer;

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub enum LockError {
    WouldDeadlock,
    WouldBlock,
}

#[derive(Debug)]
pub struct Underflow;

impl Display for LockError {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for LockError {}

impl Display for Underflow {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(f, "Display")
    }
}

impl Error for Underflow {}

async fn future_waker() -> Waker {
    poll_fn(|ctx| Poll::Ready(ctx.waker().clone())).await
}

async fn future_wait(on_cancel: impl FnOnce()) {
    let d = defer(on_cancel);
    pending!();
    mem::forget(d);
}


pub struct AssertSyncSend<F>(F);

impl<F: Future> Future for AssertSyncSend<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().0) }.poll(cx)
    }
}

unsafe impl<F> Send for AssertSyncSend<F> {}

unsafe impl<F> Sync for AssertSyncSend<F> {}

impl<F> AssertSyncSend<F> {
    unsafe fn new(x: F) -> Self {
        AssertSyncSend(x)
    }
}

//
// struct YieldOnce<F: FnOnce()>(Option<F>);
//
// impl<F: FnOnce()> Future for YieldOnce<F> {
//     type Output = ();
//
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         match self.0.take(){
//             None=>
//         }
//     }
// }
//
// impl<F: FnOnce()> Drop for YieldOnce<F> {
//     fn drop(&mut self) {
//         (self.0.take())()
//     }
// }