use std::future::Future;
use std::{io, mem};
use std::io::ErrorKind;
use std::fmt::{Display, Formatter};
use std::error::Error;
use std::fmt;
use futures::FutureExt;
use futures::pin_mut;
use futures::select;
use futures::task::{Spawn, SpawnExt};
use futures::channel::oneshot;
use ondrop::OnDrop;
use std::panic::{AssertUnwindSafe, resume_unwind};
use std::process::{exit, abort};
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use async_std::task;
use futures::executor::block_on;
use crate::promise::Promise;

#[derive(Clone, Debug)]
#[must_use]
pub struct Cancel(Promise);

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Default)]
pub struct Canceled;

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Default)]
pub struct Timeout;

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum TryCancelError {
    Timeout,
    Canceled,
}

/// A primitive for performing async cancellation of futures. Futures
/// by default support synchronous cancellation through `Drop::drop`. By accepting a `Cancel` as
/// input, a future expresses the ability to terminate early (though not immediately) after
/// `Cancel::cancel` has been called.
impl Cancel {
    pub fn new() -> Self {
        Cancel(Promise::new())
    }

    pub fn cancel(&self) {
        self.0.complete_ok();
    }

    // Wait for cancel to be called.
    pub async fn wait(&self) {
        self.0.recv_ok().await;
    }

    // Run the f until cancel is called, then drop f. This effectively wraps a synchronously
    // canceled future as an asynchronously canceled future.
    pub async fn checked<F: Future>(&self, f: F) -> Result<F::Output, Canceled> {
        let success = f.fuse();
        let failure = self.wait().fuse();
        pin_mut!(success, failure);
        select! {
            _ = failure => Err(Canceled),
            success = success => Ok(success),
        }
    }

    // Run f until a timeout after cancellation. This effectively puts a timeout on asynchronous
    // cancellation and uses synchronous cancellation after the timeout.
    pub async fn checked_timeout<F: Future>(&self, duration: Duration, f: F) -> Result<F::Output, Timeout> {
        let success = f.fuse();
        let failure = async {
            self.wait().await;
            task::sleep(duration).await;
            Err(Timeout)
        }.fuse();
        pin_mut!(success, failure);
        select! {
            failure = failure => failure?,
            success = success => Ok(success),
        }
    }

    // Spawn a remote task governed by this Cancel. Dropping the returned future will trigger cancel.
    // This effectively wraps an asynchronously canceled future as a synchronously canceled future.
    pub fn spawn<F>(
        &self,
        spawn: &dyn Spawn,
        f: F,
    ) -> impl Future<Output=Result<F::Output, Canceled>>
        where F: Future + Send + 'static,
              F::Output: Send {
        let (remote, handle) = self.remote_handle(f);
        spawn.spawn(remote).unwrap();
        handle
    }

    // pub fn spawn_timeout<F>(
    //     &self,
    //     spawn: &dyn Spawn,
    //     duration: Duration,
    //     f: F,
    // ) -> impl Future<Output=Result<F::Output, Canceled>>
    //     where F: Future + Send + 'static,
    //           F::Output: Send {
    //     let (remote, handle) = self.remote_handle(f);
    //     spawn.spawn(self.checked_timeout(remote)).unwrap();
    //     handle
    // }


    pub fn remote_handle<F>(
        &self,
        f: F,
    ) -> (
        impl Future<Output=()>,
        impl Future<Output=Result<F::Output, Canceled>>
    ) where F: Future {
        let this = self.clone();
        let (sender, receiver) = oneshot::channel();
        let cancel_guard = OnDrop::new(move || this.cancel());
        (async move {
            sender.send(AssertUnwindSafe(f).catch_unwind().await).ok();
        }, async move {
            let result = receiver.await;
            mem::drop(cancel_guard.into_inner());
            match result? {
                Ok(result) => Ok(result),
                Err(panic) => resume_unwind(panic)
            }
        })
    }

    pub fn cancel_on_control_c(&self) {
        let counter = AtomicUsize::new(0);
        let this = self.clone();
        ctrlc::set_handler(move || {
            match counter.fetch_add(1, Ordering::SeqCst) {
                0 => {
                    eprintln!("Canceling because of control-c");
                    this.clone().cancel();
                }
                1 => eprintln!("Really skip cancellation?"),
                2 => eprintln!("Really skip cancellation??"),
                3 => exit(3),
                4 => eprintln!("Really skip exit handlers?"),
                5 => eprintln!("Really skip exit handlers??"),
                _ => abort(),
            }
        }).unwrap();
    }

    pub fn run_main<E: Display>(&self, dur: Duration, f: impl Future<Output=Result<(), E>>) -> ! {
        self.cancel_on_control_c();
        block_on(async {
            match self.checked_timeout(dur, f).await {
                Ok(Ok(())) => std::process::exit(0),
                Ok(Err(internal)) => {
                    eprintln!("{}", internal);
                    std::process::exit(1);
                }
                Err(Timeout) => {
                    eprintln!("Asynchronous cancellation timeout: terminating...");
                    std::process::exit(1)
                }
            }
        })
    }
}

impl From<Canceled> for io::Error {
    fn from(x: Canceled) -> Self { io::Error::new(ErrorKind::Interrupted, x) }
}

impl From<Timeout> for io::Error {
    fn from(x: Timeout) -> Self { io::Error::new(ErrorKind::TimedOut, x) }
}

impl From<oneshot::Canceled> for Canceled {
    fn from(oneshot::Canceled: oneshot::Canceled) -> Self {
        Canceled
    }
}

impl Display for Canceled {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { write!(f, "Task canceled") }
}

impl Display for Timeout {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { write!(f, "Task canceled") }
}

impl Error for Canceled {}

impl Error for Timeout {}


#[test]
fn test_return() {
    use futures::executor::LocalPool;

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let cancel = Cancel::new();
    let handle = cancel.spawn(&spawner, {
        async move {
            1
        }
    }).shared();
    assert_eq!(None, handle.clone().now_or_never());
    pool.run_until_stalled();
    assert_eq!(Some(Ok(1)), handle.clone().now_or_never());
    pool.run();
}


#[test]
fn test_drop_handle() {
    use futures::executor::LocalPool;
    use std::iter;

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let cancel = Cancel::new();
    let (sender, receiver) = async_channel::bounded(3);
    let handle = cancel.spawn(&spawner, {
        let cancel = cancel.clone();
        async move {
            for x in 0.. {
                match cancel.checked(sender.send(x)).await {
                    Ok(x) => x.unwrap(),
                    Err(Canceled) => break,
                }
            }
            sender.send(-1).await.unwrap();
        }
    });
    pool.run_until_stalled();
    assert_eq!(vec![0, 1, 2], iter::from_fn(|| receiver.try_recv().ok()).collect::<Vec<isize>>());
    mem::drop(handle);
    assert_eq!(Vec::<isize>::new(), iter::from_fn(|| receiver.try_recv().ok()).collect::<Vec<isize>>());
    pool.run_until_stalled();
    assert_eq!(vec![-1], iter::from_fn(|| receiver.try_recv().ok()).collect::<Vec<isize>>());
    pool.run();
}