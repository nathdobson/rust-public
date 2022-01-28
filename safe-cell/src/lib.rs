#![feature(default_free_fn)]
#![allow(unused_imports)]
#![feature(const_fn_trait_bound)]
#![feature(const_fn_fn_ptr_basics)]

use std::borrow::Borrow;
use std::cell::{Cell, UnsafeCell};
use std::collections::HashMap;
use std::ops::Deref;
use std::panic::resume_unwind;
use std::sync::{Arc, Barrier};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::Ordering::Release;
use std::{mem, thread};
use std::any::{Any, TypeId};
use std::default::default;
use std::hash::Hash;
use std::thread::ThreadId;
use std::time::Duration;
use ondrop::OnDrop;
use parking_lot::{Mutex, ReentrantMutex};
use cache_map::CacheMap;

pub struct SafeOnceCell<T> {
    initializing: ReentrantMutex<Cell<bool>>,
    initialized: AtomicBool,
    value: UnsafeCell<Option<T>>,
}

pub struct SafeLazy<T, F = fn() -> T> {
    cell: SafeOnceCell<T>,
    init: Cell<Option<F>>,
}

pub struct SafeOnceCellMap<K: 'static, V: 'static> {
    map: CacheMap<K, Arc<SafeOnceCell<V>>>,
}

pub struct SafeTypeMap {
    map: SafeOnceCellMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl<T> SafeOnceCell<T> {
    pub const fn new() -> Self {
        SafeOnceCell {
            initializing: ReentrantMutex::new(Cell::new(false)),
            initialized: AtomicBool::new(false),
            value: UnsafeCell::new(None),
        }
    }
    pub fn from(x: T) -> Self {
        SafeOnceCell {
            initializing: ReentrantMutex::new(Cell::new(true)),
            initialized: AtomicBool::new(true),
            value: UnsafeCell::new(Some(x)),
        }
    }
    pub fn get(&self) -> Option<&T> {
        unsafe {
            if self.initialized.load(Ordering::Acquire) {
                return (*self.value.get()).as_ref();
            }
            None
        }
    }
    pub fn get_or_init<F: FnOnce() -> T>(&self, init: F) -> &T {
        self.get_or_init_impl(init).expect("Poisoned")
    }
    fn get_or_init_impl<F: FnOnce() -> T>(&self, init: F) -> Option<&T> {
        unsafe {
            if self.initialized.load(Ordering::Acquire) {
                return (*self.value.get()).as_ref();
            }
            let lock = self.initializing.lock();
            if self.initialized.load(Ordering::Acquire) {
                return (*self.value.get()).as_ref();
            }
            let _done = OnDrop::new(|| self.initialized.store(true, Release));
            if lock.replace(true) { panic!("Deadlock in initialization.") }
            let value = &mut *self.value.get();
            *value = Some(init());
            (*self.value.get()).as_ref()
        }
    }
    pub fn into_inner(self) -> Option<T> {
        self.value.into_inner()
    }
}

impl<T> Default for SafeOnceCell<T> {
    fn default() -> Self { SafeOnceCell::new() }
}

impl<T, F> SafeLazy<T, F> {
    pub const fn new(f: F) -> Self {
        SafeLazy { cell: SafeOnceCell::new(), init: Cell::new(Some(f)) }
    }
}

impl<T: Default> SafeLazy<T> {
    pub const fn const_default() -> Self {
        SafeLazy::new(|| T::default())
    }
}

impl<T, F: FnOnce() -> T> Deref for SafeLazy<T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.cell.get_or_init(|| {
            self.init.take().unwrap()()
        })
    }
}

unsafe impl<T: Send> Send for SafeOnceCell<T> {}

unsafe impl<T: Sync + Send> Sync for SafeOnceCell<T> {}

unsafe impl<T: Send, F: Send> Send for SafeLazy<T, F> {}

unsafe impl<T: Sync + Send, F: Send> Sync for SafeLazy<T, F> {}

impl<K: Eq + Hash, V> SafeOnceCellMap<K, V> {
    pub fn get_or_init<'a, Q, F>(&'a self, key: &Q, f: F) -> &'a V where Q: ?Sized + ToOwned<Owned=K>, K: Borrow<Q>, F: FnOnce() -> V, Q: Eq + Hash {
        self.map.get_or_init(key, default).get_or_init(f)
    }
}

impl<K, V> SafeOnceCellMap<K, V> {
    pub fn new() -> Self {
        SafeOnceCellMap { map: CacheMap::new() }
    }
}

impl SafeTypeMap {
    pub fn new() -> Self {
        SafeTypeMap { map: SafeOnceCellMap::new() }
    }
    pub fn get_or_init<'a, F, T: 'static + Send + Sync>(&'a self, f: F) -> &'a T where F: FnOnce() -> T {
        let type_id = TypeId::of::<T>();
        let result: &'a Box<dyn Any + Send + Sync> = self.map.get_or_init(&type_id, || Box::new(f()));
        result.downcast_ref::<T>().unwrap()
    }
}

impl Default for SafeTypeMap {
    fn default() -> Self {
        SafeTypeMap::new()
    }
}

impl<T: Clone> Clone for SafeOnceCell<T> {
    fn clone(&self) -> Self {
        if let Some(x) = self.get() {
            SafeOnceCell::from(x.clone())
        } else {
            SafeOnceCell::new()
        }
    }
}

#[test]
fn test_simple() {
    let foo = SafeOnceCell::new();
    assert_eq!(&42u8, foo.get_or_init(|| 42u8));
}

#[cfg(test)]
fn parallel(threads: usize, f: impl 'static + Send + Sync + Fn()) {
    let barrier = Arc::new(Barrier::new(threads));
    let f = Arc::new(f);
    (0..threads).map(|_| thread::spawn({
        let barrier = barrier.clone();
        let f = f.clone();
        move || {
            barrier.wait();
            f();
        }
    })).collect::<Vec<_>>().into_iter().for_each(|x| { x.join().unwrap_or_else(|x| resume_unwind(x)); });
}

#[test]
fn test_racy() {
    let cell = Arc::new(SafeOnceCell::new());
    parallel(1000, {
        let cell = cell.clone();
        move || {
            assert_eq!(cell.get_or_init(|| 42u8), &42u8);
        }
    });
    assert_eq!(&42, cell.get_or_init(|| panic!()));
}

#[test]
#[should_panic(expected = "Deadlock")]
fn test_reentrant() {
    let cell = SafeOnceCell::new();
    cell.get_or_init(|| *cell.get_or_init(|| 42));
}

#[test]
#[should_panic(expected = "Poisoned")]
fn test_racy_reentrant() {
    use rand::thread_rng;
    use rand::Rng;

    let cell = Arc::new(SafeOnceCell::new());
    parallel(100, {
        let cell = cell.clone();
        move || {
            cell.get_or_init(|| {
                thread::sleep(Duration::from_millis(thread_rng().gen_range(0..100)));
                *cell.get_or_init(|| 42)
            });
        }
    });
}

#[test]
fn test_map() {
    let map = SafeOnceCellMap::<String, String>::new();
    assert_eq!(map.get_or_init("a", || "b".to_string()), "b");
    assert_eq!(map.get_or_init("a", || "c".to_string()), "b");
    assert_eq!(map.get_or_init("x", || {
        assert_eq!(map.get_or_init("y", || {
            "y".to_string()
        }), "y");
        "x".to_string()
    }), "x");
}

#[test]
#[should_panic(expected = "Deadlock")]
fn test_map_reentrant() {
    let map = SafeOnceCellMap::<String, String>::new();
    map.get_or_init("x", || {
        map.get_or_init("x", || {
            "x".to_string()
        });
        "x".to_string()
    });
}