use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Condvar, LockResult, Mutex, MutexGuard, TryLockError, TryLockResult};

pub struct State<T: ?Sized> {
    version: usize,
    writers: usize,
    value: T,
}

struct Inner<T: ?Sized> {
    condvar: Condvar,
    mutex: Mutex<State<T>>,
}

pub struct Watchable<T: ?Sized> {
    inner: Arc<Inner<T>>,
}

pub struct Watch<T: ?Sized> {
    version: usize,
    inner: Arc<Inner<T>>,
}

impl<T: ?Sized> Watchable<T> {
    pub fn new(value: T) -> Self
    where
        T: Sized,
    {
        Watchable {
            inner: Arc::new(Inner {
                mutex: Mutex::new(State {
                    version: 1,
                    value,
                    writers: 1,
                }),
                condvar: Condvar::new(),
            }),
        }
    }
    pub fn lock(&self) -> LockResult<MutexGuard<State<T>>> {
        let mut lock = self.inner.mutex.lock()?;
        lock.version += 1;
        self.inner.condvar.notify_all();
        Ok(lock)
    }
    pub fn watch(&self) -> Watch<T> {
        Watch {
            inner: self.inner.clone(),
            version: 0,
        }
    }
}

impl<T> Watch<T> {
    pub fn next(&mut self) -> TryLockResult<MutexGuard<State<T>>> {
        let mut lock = self.inner.mutex.lock()?;
        loop {
            if self.version != lock.version {
                self.version = lock.version;
                return Ok(lock);
            }
            if lock.writers == 0 {
                return Err(TryLockError::WouldBlock);
            }
            lock = self.inner.condvar.wait(lock)?;
        }
    }
}

impl<T: ?Sized> Drop for Watchable<T> {
    fn drop(&mut self) {
        let mut lock = self.inner.mutex.lock().unwrap();
        lock.writers -= 1;
        if lock.writers == 0 {
            self.inner.condvar.notify_all();
        }
    }
}

impl<T: ?Sized> Clone for Watchable<T> {
    fn clone(&self) -> Self {
        self.inner.mutex.lock().unwrap().writers += 1;
        Watchable {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Deref for State<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.value }
}

impl<T> DerefMut for State<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.value }
}

impl<T> Clone for Watch<T> {
    fn clone(&self) -> Self {
        Watch {
            version: self.version,
            inner: self.inner.clone(),
        }
    }
}
