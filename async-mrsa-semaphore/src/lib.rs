#![feature(future_poll_fn)]

use std::sync::Arc;
use futures::task::AtomicWaker;
use std::future::poll_fn;
use std::sync::atomic::{Ordering, AtomicI64};
use std::task::Poll;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
struct Inner {
    available: AtomicI64,
    waker: AtomicWaker,
}

#[derive(Debug)]
pub struct Acquirer(Arc<Inner>);

#[derive(Debug)]
pub struct Releaser(Arc<Inner>);


#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub struct AcquireError;

impl Display for AcquireError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
}

impl Error for AcquireError {}

pub const SEMAPHORE_MAX: u64 = i64::MAX as u64;

pub fn semaphore(initial: u64) -> (Acquirer, Releaser) {
    assert!(initial <= i64::MAX as u64);
    let inner = Arc::new(Inner {
        available: AtomicI64::new(initial as i64),
        waker: AtomicWaker::new(),
    });
    (Acquirer(inner.clone()), Releaser(inner.clone()))
}

impl Acquirer {
    pub fn try_acquire(&mut self, acquire: u64) -> Result<bool, AcquireError> {
        assert!(acquire <= i64::MAX as u64);
        let acquire = acquire as i64;
        let available = self.0.available.load(Ordering::Acquire);
        if acquire <= available {
            self.0.available.fetch_sub(acquire, Ordering::AcqRel);
            Ok(true)
        } else {
            if available == i64::MIN {
                Err(AcquireError)
            } else {
                Ok(false)
            }
        }
    }
    pub async fn acquire(&mut self, acquire: u64) -> Result<(), AcquireError> {
        poll_fn(|cx| {
            if self.try_acquire(acquire)? {
                return Poll::Ready(Ok(()));
            }
            self.0.waker.register(cx.waker());
            if self.try_acquire(acquire)? {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }).await
    }
}

impl Releaser {
    pub fn release(&self, release: u64) {
        assert!(release <= i64::MAX as u64);
        let release = release as i64;
        self.0.available.fetch_add(release, Ordering::AcqRel).checked_add(release).expect("overflowing semaphore");
        self.0.waker.wake();
    }
}

impl Drop for Releaser {
    fn drop(&mut self) {
        self.0.available.store(i64::MIN, Ordering::Release);
        self.0.waker.wake();
    }
}

#[cfg(test)]
mod test {
    use crate::semaphore;
    use std::time::Duration;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn test() {
        let (mut acquirer, releaser) = semaphore(1);
        acquirer.acquire(1).await;
        static DONE: AtomicBool = AtomicBool::new(false);
        let joiner = tokio::spawn(async move {
            acquirer.acquire(1).await;
            DONE.store(true, Ordering::SeqCst);
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!DONE.load(Ordering::SeqCst));
        releaser.release(1);
        joiner.await.unwrap();
    }
}