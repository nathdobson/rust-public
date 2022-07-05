use std::io::Take;
use std::marker::Unsize;
use std::sync::{Arc, Weak};

// use crate::take_cell::TakeCell;
// use crate::weak_vec::WeakVec;

pub struct TakeBag<U: ?Sized>(WeakVec<TakeCell<U>>);

pub struct Guard<U: ?Sized>(Arc<TakeCell<U>>);

impl<U: ?Sized> TakeBag<U> {
    pub fn new() -> Self { TakeBag(WeakVec::new()) }
    pub fn push<T: Unsize<U>>(&mut self, x: T) -> Guard<U> {
        Guard(self.0.push_new::<TakeCell<T>>(TakeCell::new(x)))
    }
    pub fn drain<'a>(&'a mut self) -> impl 'a + Iterator<Item = U>
    where
        U: Sized,
    {
        self.0.drain().filter_map(|x| x.take())
    }
}

impl<U: ?Sized + FnOnce()> TakeBag<U> {
    pub fn drain_call(&mut self) {
        for x in self.0.drain() {
            x.call()
        }
    }
}
