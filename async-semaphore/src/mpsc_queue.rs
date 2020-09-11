use crate::atomic::Atomic;
use std::ops::Deref;
use std::sync::atomic::{AtomicU128, AtomicUsize, AtomicU64};
use std::mem::MaybeUninit;
use crate::mpmc::Waiter;

impl<T> Stack<T> {
    pub fn new() -> Self {
        unimplemented!()
    }
    pub fn push_all(&self, node: Box<Node<T>>) {
        unimplemented!()
    }
    pub fn pop(&self) -> Option<Box<Node<T>>> {
        unimplemented!()
    }
    pub fn take(&self) -> Option<Box<Node<T>>> {
        unimplemented!()
    }
}

impl<T> Drop for Stack<T> {
    fn drop(&mut self) {
        unimplemented!()
    }
}

impl<T> Drop for Node<T> {
    fn drop(&mut self) {
        unimplemented!()
    }
}


impl<T> Queue<T> {
    pub fn new() -> Self {
        unimplemented!()
    }
    pub fn push_back(&self, value: T) -> NodeRef<T> {
        unimplemented!()
    }
    pub fn front(&self) -> Option<&'_ Node<T>> {
        unimplemented!()
    }
    pub fn pop_front(&self) -> Option<NodeRef<T>> {
        unimplemented!()
    }
}