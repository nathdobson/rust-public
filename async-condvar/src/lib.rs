#![feature(negative_impls)]
#![feature(future_poll_fn)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(unused_must_use)]

use parking_lot::Mutex;
use parking_lot::MutexGuard;
use std::future::Future;
use core::mem;
use std::ptr::null;
use std::collections::VecDeque;
use std::task::{Waker, Poll, Context};
use std::cell::UnsafeCell;
use std::future::poll_fn;
use std::pin::Pin;
use std::thread;
use std::sync::Arc;

pub struct WakerGuard(Waker);

#[must_use]
pub struct Notifier(Option<Waker>);

impl ! Send for WakerGuard {}

impl WakerGuard {
    pub fn new() -> impl Future<Output=Self> + Send {
        poll_fn(|cx| {
            Poll::Ready(WakerGuard(cx.waker().clone()))
        })
    }
}

#[derive(Debug)]
struct CondvarState {
    mutex: usize,
    start: u64,
    waiters: VecDeque<Option<Waker>>,
}

pub struct Condvar {
    state: Mutex<CondvarState>,
}

struct WaitFuture<'a, T> {
    started: bool,
    mutex: Option<&'a Mutex<T>>,
    condvar: &'a Condvar,
    index: u64,
}

impl<'a, T> Unpin for WaitFuture<'a, T> {}

impl<'a, T> Future for WaitFuture<'a, T> {
    type Output = (WakerGuard, MutexGuard<'a, T>);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.started {
            self.started = true;
            return Poll::Pending;
        }
        let guard = self.mutex.unwrap().lock();
        let mut state = self.condvar.state.try_lock().unwrap();
        let state = &mut *state;
        if state.start <= self.index {
            state.waiters[(self.index - state.start) as usize] = Some(cx.waker().clone());
            Poll::Pending
        } else {
            self.mutex = None;
            Poll::Ready((WakerGuard(cx.waker().clone()), guard))
        }
    }
}

impl<'a, T> Drop for WaitFuture<'a, T> {
    fn drop(&mut self) {
        if let Some(mutex) = self.mutex.as_mut() {
            let guard = mutex.lock();
            let state = &mut *self.condvar.state.try_lock().unwrap();
            if state.start < self.index {
                state.waiters[(self.index - state.start) as usize] = None;
            } else {
                state.notify(1);
            }
        }
    }
}

impl Notifier {
    pub fn notify(self) {
        if let Some(waker) = self.0 {
            waker.wake()
        }
    }
}

impl CondvarState {
    pub fn notify(&mut self, mut count: usize) {
        while count > 0 {
            self.start += 1;
            match self.waiters.pop_front() {
                None => break,
                Some(None) => continue,
                Some(Some(x)) => {
                    x.wake();
                    count -= 1;
                }
            }
        }
    }
    #[must_use]
    pub fn notify_one(&mut self) -> Notifier {
        self.start += 1;
        while let Some(x) = self.waiters.pop_front() {
            if let Some(x) = x {
                return Notifier(Some(x));
            }
        }
        Notifier(None)
    }
    fn check_mutex<T>(&mut self, mutex: &Mutex<T>) {
        let mutex_int: usize = mutex as *const Mutex<T> as usize;
        if self.mutex == 0 {
            self.mutex = mutex_int;
        } else {
            assert_eq!(self.mutex, mutex_int);
        }
    }
}

impl Condvar {
    pub fn new() -> Self {
        Condvar {
            state: Mutex::new(CondvarState {
                mutex: 0,
                start: 0,
                waiters: Default::default(),
            }),
        }
    }
    pub fn wait<'a, T: Send>(
        &'a self,
        waker: WakerGuard,
        guard: MutexGuard<'a, T>,
    ) -> impl Future<Output=(WakerGuard, MutexGuard<'a, T>)> + Send + 'a {
        let mutex: &'a Mutex<T> = MutexGuard::mutex(&guard);

        let index;
        {
            let mut state = self.state.try_lock().unwrap();
            state.check_mutex(mutex);
            index = state.start + state.waiters.len() as u64;
            state.waiters.push_back(Some(waker.0))
        }
        mem::drop(guard);
        WaitFuture {
            started: false,
            condvar: self,
            mutex: Some(mutex),
            index: index,
        }
    }
    #[must_use]
    pub fn notify<'a, T>(&self, guard: &mut MutexGuard<'a, T>, count: usize) {
        let mutex: &'a Mutex<T> = MutexGuard::mutex(&guard);
        let mut state = self.state.try_lock().unwrap();
        state.check_mutex(mutex);
        state.notify(count);
    }
    pub fn notify_one<'a, T>(&self, guard: &mut MutexGuard<'a, T>, count: usize) -> Notifier {
        let mutex: &'a Mutex<T> = MutexGuard::mutex(&guard);
        let mut state = self.state.try_lock().unwrap();
        state.check_mutex(mutex);
        state.notify_one()
    }
    pub fn lock_when<'a, T: Send, O: Send, F: 'a + Send + FnMut(&mut T) -> Option<O>>(
        &'a self,
        mutex: &'a Mutex<T>,
        mut cond: F,
    ) -> impl Future<Output=(MutexGuard<'a, T>, O)> + Send + 'a {
        async move {
            let mut fut = None;
            loop {
                if let Some(fut2) = fut {
                    let (waker, mut lock): (WakerGuard, MutexGuard<'a, T>) = fut2.await;
                    if let Some(result) = cond(&mut *lock) {
                        return (lock, result);
                    }
                    fut = Some(self.wait(waker, lock));
                } else {
                    let waker = WakerGuard::new().await;
                    let mut lock = mutex.lock();
                    if let Some(result) = cond(&mut *lock) {
                        return (lock, result);
                    }
                    fut = Some(self.wait(waker, lock))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use parking_lot::Mutex;
    use parking_lot::MutexGuard;
    use std::future::Future;
    use std::sync::Arc;
    use tokio;
    use crate::Condvar;

    #[tokio::test]
    async fn test() {
        test_impl().await
    }

    fn test_impl() -> impl Future<Output=()> + Send {
        async {
            let p = Arc::new((Mutex::new(0), Condvar::new()));
            let t1 = tokio::spawn({
                let p = p.clone();
                async move {
                    for i in 0..5 {
                        let mut lock = p.1.lock_when(&p.0, |x| (*x) % 2 == 0).await;
                        *lock += 1;
                        p.1.notify(&mut lock, 1);
                    }
                }
            });
            for i in 0..5 {
                let mut lock = p.1.lock_when(&p.0, |x| (*x) % 2 == 1).await;
                *lock += 1;
                p.1.notify(&mut lock, 1);
            }
            t1.await.unwrap();
            assert_eq!(*p.0.lock(), 10);
        }
    }
}
