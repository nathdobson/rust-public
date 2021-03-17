use std::marker::Unsize;
use std::ops::{Index, IndexMut, CoerceUnsized};
use std::sync::atomic::AtomicU64;
use std::sync::{atomic, Arc};
use crate::shared::Shared;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::cell::UnsafeCell;
use std::sync::atomic::Ordering::Relaxed;

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct SharedCell { id: u64 }

struct Inner<A: ?Sized> {
    id: u64,
    value: UnsafeCell<A>,
}

pub struct Key<A: ?Sized + 'static>(Arc<Inner<A>>);

impl SharedCell {
    pub fn new() -> Self { SharedCell { id: COUNTER.fetch_add(1, Relaxed) } }
    pub fn push<A: 'static>(&self, value: A) -> Key<A> {
        Key(Arc::new(Inner { id: self.id, value: UnsafeCell::new(value) }))
    }
}

impl<'t, A: 'static + ?Sized, > Index<&'t Key<A>> for SharedCell {
    type Output = A;
    fn index<'b>(&'b self, key: &'t Key<A>) -> &'b A {
        unsafe {
            assert!(self.id == key.0.id);
            &*key.0.value.get()
        }
    }
}

impl<'t, A: 'static + ?Sized, > IndexMut<&'t Key<A>> for SharedCell {
    fn index_mut<'b>(&'b mut self, key: &'t Key<A>) -> &'b mut A {
        unsafe {
            assert!(self.id == key.0.id);
            &mut *key.0.value.get()
        }
    }
}

impl<A: 'static + ?Sized> Eq for Key<A> {}

impl<A: 'static + ?Sized> PartialEq for Key<A> {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.0).eq(&Arc::as_ptr(&other.0))
    }
}

impl<A: 'static + ?Sized> Ord for Key<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        Arc::as_ptr(&self.0).cmp(&Arc::as_ptr(&other.0))
    }
}

impl<A: 'static + ?Sized> PartialOrd for Key<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Arc::as_ptr(&self.0).partial_cmp(&Arc::as_ptr(&other.0))
    }
}

impl<A: 'static + ?Sized> Hash for Key<A> {
    fn hash<H: Hasher>(&self, state: &mut H) { Arc::as_ptr(&self.0).hash(state); }
}

impl<A: 'static + ?Sized> Clone for Key<A> {
    fn clone(&self) -> Self { Key(self.0.clone()) }
}

impl<T, U> CoerceUnsized<Key<U>> for Key<T> where
    T: Unsize<U> + ?Sized,
    U: ?Sized {}