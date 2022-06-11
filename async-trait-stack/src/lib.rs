#![feature(allocator_api)]

use std::alloc::{AllocError, Allocator, Layout};
use std::cell::{Cell, UnsafeCell};
use std::mem::MaybeUninit;
use std::ptr::NonNull;

#[repr(align(16))]
struct InlineAllocator<T> {
    allocated: Cell<bool>,
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> InlineAllocator<T> {
    pub fn new() -> Self {
        InlineAllocator {
            allocated: Cell::new(false),
            value: UnsafeCell::new(MaybeUninit::new()),
        }
    }
}
