use std::marker::Unsize;
use std::ops::{Index, IndexMut, CoerceUnsized};
use std::sync::atomic::AtomicU64;
use std::sync::atomic;
use crate::shared::Shared;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct Bag {
    id: u64,
}

struct Inner<A: ?Sized> {
    id: u64,
    value: A,
}

pub struct Token<A: ?Sized + 'static> {
    inner: Shared<Inner<A>>
}

impl Bag {
    pub fn new() -> Self {
        Bag {
            id: COUNTER.fetch_add(1, atomic::Ordering::SeqCst)
        }
    }
    pub fn push<A: 'static>(&self, value: A) -> Token<A> {
        Token { inner: Shared::new(Inner { id: self.id, value }) }
    }
}

impl<'t, A: 'static + ?Sized, > Index<&'t Token<A>> for Bag {
    type Output = A;
    fn index<'b>(&'b self, key: &'t Token<A>) -> &'b A {
        unsafe {
            assert!(self.id == key.inner.id);
            &*(&key.inner.value as *const A)
        }
    }
}

impl<'t, A: 'static + ?Sized, > IndexMut<&'t Token<A>> for Bag {
    fn index_mut<'b>(&'b mut self, key: &'t Token<A>) -> &'b mut A {
        unsafe {
            assert!(self.id == key.inner.id);
            &mut *(&key.inner.value as *const A as *mut A)
        }
    }
}

impl<A: 'static + ?Sized> Eq for Token<A> {}

impl<A: 'static + ?Sized> PartialEq for Token<A> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<A: 'static + ?Sized> Ord for Token<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<A: 'static + ?Sized> PartialOrd for Token<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<A: 'static + ?Sized> Hash for Token<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<A: 'static + ?Sized> Clone for Token<A> {
    fn clone(&self) -> Self {
        Token { inner: self.inner.clone() }
    }
}

impl<T, U> CoerceUnsized<Token<U>> for Token<T> where
    T: Unsize<U> + ?Sized,
    U: ?Sized {}