use std::sync::{Arc, Mutex, Weak};
use crate::weak_vec::WeakVec;
use std::ops::{Deref, DerefMut};
use std::any::Any;
use std::mem;

pub struct Listenable<T> {
    listeners: WeakVec<dyn Fn(&mut T)>,
    inner: T,
}

pub struct WriteGuard<'a, T>(&'a mut Listenable<T>);

#[must_use]
pub struct ListenGuard(Arc<dyn Any>);

impl<T> Listenable<T> {
    pub fn new(inner: T) -> Self { Listenable { listeners: WeakVec::new(), inner } }
    pub fn write(&mut self) -> WriteGuard<T> { WriteGuard(self) }
    pub fn listen(&mut self, listener: impl Fn(&mut T) + 'static) -> ListenGuard {
        let listener = Arc::new(listener);
        self.listeners.push(Arc::downgrade(&listener) as _);
        ListenGuard(listener)
    }
}

impl<T> Deref for Listenable<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<'a, T> Deref for WriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &*self.0 }
}

impl<'a, T> DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.inner
    }
}

impl<'a, T> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        for x in self.0.listeners.drain() {
            x(&mut self.0.inner)
        }
    }
}

#[test]
fn test() {
    let mut listen = Listenable::new(0);
    let listener = listen.listen(|x| *x = *x & !1);
    *listen.write() = 3;
    assert_eq!(*listen, 2);
    mem::drop(listener);
    *listen.write() = 3;
    assert_eq!(*listen, 3);
}