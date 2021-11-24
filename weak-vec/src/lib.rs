#![feature(unsize)]
#![feature(vec_into_raw_parts)]

use std::sync::{Weak, Arc};
use std::mem;
use take_cell::TakeCell;
use std::marker::Unsize;

const MAX_FACTOR: f64 = 0.5;

pub struct WeakVec<T: ?Sized> {
    vec: Vec<Weak<T>>,
    potential: usize,
}

impl<T: ?Sized> WeakVec<T> {
    pub fn new() -> WeakVec<T> {
        WeakVec { vec: vec![], potential: 0 }
    }
    pub fn push(&mut self, element: Weak<T>) {
        if self.vec.len() == self.vec.capacity() && (self.potential as f64 / self.vec.len() as f64) >= MAX_FACTOR {
            self.iter().count();
        }
        self.vec.push(element);
        self.potential += 1;
    }
    pub fn push_new<T2: Unsize<T>>(&mut self, element: T2) -> Arc<T2> {
        let element = Arc::new(element);
        self.push(Arc::downgrade(&element) as Weak<T>);
        element
    }
    pub fn iter<'a>(&'a mut self) -> impl Iterator<Item=Arc<T>> + 'a {
        self.potential = 0;
        self.vec.filter_iter(|x| x.upgrade().ok_or(())).filter_map(|x| x.ok())
    }
    pub fn drain<'a>(&'a mut self) -> impl Iterator<Item=Arc<T>> + 'a {
        self.vec.drain(..).filter_map(|x| x.upgrade())
    }
}

impl<T: FnOnce()> WeakVec<TakeCell<T>> {
    pub fn drain_call(&mut self) {
        for x in self.drain() {
            x.call()
        }
    }
}

impl<T: Fn() + ?Sized> WeakVec<T> {
    pub fn call(&mut self) {
        for x in self.iter() {
            x()
        }
    }
}

trait VecExt {
    type Element;
    fn filter_iter<'a, S, E, F>(&'a mut self, f: F) -> FilterIter<'a, Self::Element, S, E, F>
        where F: FnMut(&'a mut Self::Element) -> Result<S, E>;
}

impl<T> VecExt for Vec<T> {
    type Element = T;

    fn filter_iter<'a, S, E, F>(&'a mut self, fun: F) -> FilterIter<'a, T, S, E, F>
        where F: FnMut(&'a mut Self::Element) -> Result<S, E> {
        FilterIter::new(self, fun)
    }
}

struct FilterIter<'a, T, S, E, F: FnMut(&'a mut T) -> Result<S, E>> {
    vec: &'a mut Vec<T>,
    fun: F,
    cap: usize,
    ptr: *mut T,
    next_write: *mut T,
    next_read: *mut T,
    end: *mut T,
    panic_flag: bool,
}

impl<'a, T, S, E, F: FnMut(&'a mut T) -> Result<S, E>> FilterIter<'a, T, S, E, F> {
    fn new(vec: &'a mut Vec<T>, fun: F) -> Self {
        unsafe {
            let (ptr, len, cap) = mem::replace(vec, vec![]).into_raw_parts();
            FilterIter {
                vec: vec,
                ptr,
                cap,
                fun,
                next_write: ptr,
                next_read: ptr,
                end: ptr.offset(len as isize),
                panic_flag: false,
            }
        }
    }
    pub fn ignore(&mut self) {
        unsafe {
            while self.next_read != self.end {
                self.next_write.write(self.next_read.read());
                self.next_read = self.next_read.offset(1);
                self.next_write = self.next_write.offset(1);
            }
        }
    }
}

impl<'a, T, S, E, F: FnMut(&'a mut T) -> Result<S, E>> Drop for FilterIter<'a, T, S, E, F> {
    fn drop(&mut self) {
        struct OnDrop<'b, 'a, T, S, E, F: FnMut(&'a mut T) -> Result<S, E>>(&'b mut FilterIter<'a, T, S, E, F>);
        impl<'b, 'a, T, S, E, F: FnMut(&'a mut T) -> Result<S, E>> Drop for OnDrop<'b, 'a, T, S, E, F> {
            fn drop(&mut self) {
                unsafe {
                    self.0.ignore();
                    let len = self.0.next_write.offset_from(self.0.ptr);
                    *self.0.vec = Vec::from_raw_parts(self.0.ptr, len as usize, self.0.cap);
                }
            }
        }
        let on_drop = OnDrop(self);
        if !on_drop.0.panic_flag {
            on_drop.0.for_each(mem::drop);
        }
    }
}

impl<'a, T, S, E, F: FnMut(&'a mut T) -> Result<S, E>> Iterator for FilterIter<'a, T, S, E, F> {
    type Item = Result<S, (T, E)>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.next_read == self.end {
                return None;
            }
            self.next_write.write(self.next_read.read());
            let old_write = self.next_write;
            self.next_read = self.next_read.offset(1);
            self.next_write = self.next_write.offset(1);
            self.panic_flag = true;
            let result = (self.fun)(&mut *old_write);
            self.panic_flag = false;
            match result {
                Ok(success) => {
                    return Some(Ok(success));
                }
                Err(error) => {
                    self.next_write = self.next_write.offset(-1);
                    return Some(Err((old_write.read(), error)));
                }
            }
        }
    }
}

#[test]
fn test_weak_bag() {
    let mut bag = WeakVec::new();
    let x1 = Arc::new(1);
    let x2 = Arc::new(2);
    bag.push(Arc::downgrade(&x1));
    bag.push(Arc::downgrade(&x2));
    assert_eq!(bag.iter().map(|x| *x).collect::<Vec<_>>(), vec![1, 2]);
    mem::drop(x1);
    assert_eq!(bag.iter().map(|x| *x).collect::<Vec<_>>(), vec![2]);
    mem::drop(x2);
    assert_eq!(bag.iter().map(|x| *x).collect::<Vec<_>>(), vec![0; 0]);
}