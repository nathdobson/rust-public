use std::ops::{Deref, DerefMut};
use std::{mem, thread};

use crate::watch::Watchable;

pub struct Dirty<T: ?Sized> {
    dirty: bool,
    value: T,
}

impl<T: ?Sized> Dirty<T> {
    pub fn new(value: T) -> Self
    where
        T: Sized,
    {
        Dirty {
            dirty: false,
            value,
        }
    }
    pub fn check_dirty(&mut self) -> bool { mem::replace(&mut self.dirty, false) }
}

impl<T: ?Sized> Deref for Dirty<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.value }
}

impl<T: ?Sized> DerefMut for Dirty<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.dirty = true;
        &mut self.value
    }
}

pub fn dirty_loop<T: Send + 'static>(
    value: T,
    mut callback: Box<dyn FnMut(&mut T) + 'static + Send>,
) -> Watchable<T> {
    let watch = Watchable::new(value);
    let mut reader = watch.watch();
    thread::spawn(move || {
        while let Ok(mut lock) = reader.next() {
            callback(&mut **lock)
        }
    });
    watch
}

#[test]
fn test_dirty() {
    use std::sync::{Arc, Barrier};

    let b1 = Arc::new(Barrier::new(2));
    let b2 = b1.clone();
    let dirty = dirty_loop(
        (),
        Box::new(move |&mut ()| {
            b2.wait();
            b2.wait();
        }),
    );
    let _ = dirty.lock().unwrap();
    b1.wait();
    let _ = dirty.lock().unwrap();
    let _ = dirty.lock().unwrap();
    b1.wait();
    b1.wait();
    b1.wait();
}

#[test]
fn test_dirty_many() {
    for _ in 0..100000 {
        test_dirty();
    }
}
