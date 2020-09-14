#![allow(unused_imports)]
#![feature(integer_atomics)]
#![feature(is_sorted)]
#![feature(test)]
#![feature(result_contains_err)]
extern crate test;

mod util;
mod atomic;
pub mod shared;
mod waker;
mod freelist;

//pub mod local;

use std::fmt::Display;
use std::error::Error;
use std::future::Future;
use std::task::{Waker, Poll};
use std::pin::Pin;
use std::mem;

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct WouldBlock;

impl Error for WouldBlock {}

impl Display for WouldBlock {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

