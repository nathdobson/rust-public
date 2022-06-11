#![feature(once_cell)]
#![feature(never_type)]

use std::future::Future;
use std::lazy::SyncOnceCell;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{RecvError, TryRecvError};
use std::sync::Arc;

use async_weighted_semaphore::{Semaphore, TryAcquireError};

#[derive(Debug)]
pub struct Promise<T = ()>(Arc<Inner<T>>);

#[derive(Debug)]
pub struct Receiver<T = ()>(Arc<Inner<T>>);

#[derive(Debug)]
struct Inner<T> {
    refcount: AtomicUsize,
    ready: Semaphore,
    value: SyncOnceCell<T>,
}

async fn recv<T>(inner: &Arc<Inner<T>>) -> Result<&T, RecvError> {
    inner.ready.acquire(1).await.unwrap_err();
    inner.value.get().ok_or(RecvError)
}

impl<T> Promise<T> {
    pub fn new() -> Self {
        Promise(Arc::new(Inner {
            refcount: AtomicUsize::new(1),
            ready: Semaphore::new(0),
            value: SyncOnceCell::new(),
        }))
    }
    pub fn complete(&self, value: T) -> Result<(), T>
    where
        T: Sized,
    {
        self.0.value.set(value)?;
        self.0.ready.poison();
        Ok(())
    }
    pub fn receiver(&self) -> Receiver<T> { Receiver(self.0.clone()) }
    pub async fn recv(&self) -> Result<&T, RecvError> { recv(&self.0).await }
}

impl Promise<!> {
    pub async fn recv_none(self) {
        let receiver = self.receiver();
        mem::drop(self);
        receiver.recv().await.err();
    }
    pub fn outlive<F: Future>(&self, f: F) -> impl Future<Output = F::Output> {
        let this = self.clone();
        async move {
            let result = f.await;
            mem::drop(this);
            result
        }
    }
    pub async fn join(self) {
        let receiver = self.receiver();
        mem::drop(self);
        receiver.recv_none().await;
    }
}

impl Receiver<!> {
    pub async fn recv_none(&self) { self.recv().await.err(); }
}

impl Promise<()> {
    pub fn complete_ok(&self) { self.complete(()).ok(); }
    pub async fn recv_ok(&self) -> bool { self.recv().await.is_ok() }
}

impl Receiver<()> {
    pub async fn recv_ok(&self) -> bool { self.recv().await.is_ok() }
}

impl<T> Receiver<T> {
    pub async fn recv(&self) -> Result<&T, RecvError> { recv(&self.0).await }
    pub fn try_recv(&self) -> Result<&T, TryRecvError> {
        match self.0.ready.try_acquire(1).unwrap_err() {
            TryAcquireError::WouldBlock => Err(TryRecvError::Empty),
            TryAcquireError::Poisoned => self.0.value.get().ok_or(TryRecvError::Disconnected),
        }
    }
}

impl<T> Clone for Promise<T> {
    fn clone(&self) -> Self {
        self.0.refcount.fetch_add(1, Ordering::Relaxed);
        Promise(self.0.clone())
    }
}

impl<T> Drop for Promise<T> {
    fn drop(&mut self) {
        if 0 == self.0.refcount.fetch_sub(1, Ordering::Relaxed) - 1 {
            self.0.ready.poison();
        }
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self { Receiver(self.0.clone()) }
}

#[cfg(test)]
mod test {
    use std::mem;
    use std::sync::mpsc::RecvError;

    use async_future_ext::FutureExt;

    use crate::Promise;

    #[tokio::test]
    async fn test_success() {
        let promise = Promise::<usize>::new();
        #[allow(unused_must_use)]
        {
            promise.receiver().clone();
        }
        assert!(promise.recv().ready().is_none());
        assert_eq!(Ok(()), promise.complete(1));
        assert_eq!(Err(2), promise.complete(2));
        assert_eq!(Ok(&1), promise.recv().ready().unwrap());
    }

    #[test]
    fn test_failure() {
        let promise = Promise::<usize>::new();
        let receiver = promise.clone().receiver();
        assert!(receiver.recv().ready().is_none());
        mem::drop(promise);
        assert_eq!(Err(RecvError), receiver.recv().ready().unwrap());
    }
}
