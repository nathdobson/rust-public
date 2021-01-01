#![feature(allocator_api,fn_traits,raw)]

//mod fiber;
//mod heap;
//mod atomic;
mod schedule;

#[macro_use]
extern crate lazy_static;

use ::libc::setcontext;
use ::libc::getcontext;
use ::libc::ucontext_t;
use ::libc::stack_t;
use std::ptr::null_mut;
use std::alloc::{GlobalAlloc, Layout, System, AllocRef};
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::sync::atomic::Ordering::SeqCst;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::thread::ThreadId;
use std::collections::HashMap;
use std::thread;
