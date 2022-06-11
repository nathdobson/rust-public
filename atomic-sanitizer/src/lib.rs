#![feature(allocator_api, fn_traits, raw)]

#[macro_use]
extern crate lazy_static;

use std::alloc::{AllocRef, GlobalAlloc, Layout, System};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Mutex;
use std::thread;
use std::thread::ThreadId;

use ::libc::{getcontext, setcontext, stack_t, ucontext_t};

//mod fiber;
//mod heap;
//mod atomic;
mod schedule;
