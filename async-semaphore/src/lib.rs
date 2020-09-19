#![allow(unused_imports, incomplete_features)]
#![feature(integer_atomics)]
#![feature(is_sorted)]
#![feature(test)]
#![feature(result_contains_err)]
#![feature(wake_trait)]
#![feature(cfg_target_has_atomic)]
extern crate test;

mod util;
mod atomic;
pub mod shared_dwcas;
pub mod shared_mutex;
//pub mod local;
//pub mod shared;
mod waker;
mod freelist;
mod queue;
#[cfg(test)]
mod bench;
//pub mod local;
#[cfg(test)]
mod profile;

use std::fmt::Display;
use std::error::Error;
use std::future::Future;
use std::task::{Waker, Poll};
use std::pin::Pin;
use std::{mem, thread};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::borrow::Borrow;

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct WouldBlock;

impl Error for WouldBlock {}

impl Display for WouldBlock {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub trait Releaser {
    fn release(&self, amount: u64);
}

pub struct ReleaseGuard<P, R> where P: Borrow<R>, R: Releaser {
    releaser: P,
    amount: u64,
    phantom: PhantomData<R>,
}

impl<P, R> ReleaseGuard<P, R> where P: Borrow<R>, R: Releaser {
    pub fn new(releaser: P, amount: u64) -> Self {
        ReleaseGuard { releaser, amount, phantom: PhantomData }
    }
    pub fn forget(self) {
        mem::forget(self)
    }
}

impl<'a, R> ReleaseGuard<&'a R, R> where R: Releaser + Clone {
    pub fn cloned(self) -> ReleaseGuard<R, R> {
        let result = ReleaseGuard {
            releaser: self.releaser.clone(),
            amount: self.amount,
            phantom: PhantomData,
        };
        mem::forget(self);
        result
    }
}


impl<P, R> Drop for ReleaseGuard<P, R> where P: Borrow<R>, R: Releaser {
    fn drop(&mut self) {
        self.releaser.borrow().release(self.amount);
    }
}
