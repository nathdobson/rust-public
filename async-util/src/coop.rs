use std::future::Future;
use std::{io, mem, thread};
use std::io::ErrorKind;
use std::fmt::{Display, Formatter, Debug};
use std::error::Error;
use std::fmt;
use ondrop::OnDrop;
use std::panic::{AssertUnwindSafe, resume_unwind};
use std::process::{exit, abort};
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::promise::Promise;
use tokio::pin;
use tokio::select;
use tokio::time::sleep;
use tokio::sync::oneshot;
use crate::spawn::{Spawn};
use tokio::task::yield_now;
use tokio::sync::mpsc::channel;
use tokio::runtime::Handle;
use std::task::{Poll, Context};
use std::future::poll_fn;
use crate::futureext::FutureExt;
use crate::join::{RemoteJoinHandle, Remote, JoinHandle};
use std::pin::Pin;
use pin_project::pin_project;
use std::sync::{Arc, Mutex};
use async_weighted_semaphore::Semaphore;


pub struct CancelInner {
    semaphore: Semaphore,
    listeners: Mutex<Vec<Box<dyn 'static + Send + FnOnce()>>>,
}

#[derive(Clone)]
#[must_use]
pub struct Cancel(Arc<CancelInner>);

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Default)]
pub struct Canceled;

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Default)]
pub struct Timeout;

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum TryCancelError {
    Timeout,
    Canceled,
}

#[pin_project]
pub struct CoopJoinHandle<J: JoinHandle> {
    cancel: Cancel,
    #[pin]
    inner: J,
}

/// A primitive for performing async cancellation of futures. Futures
/// by default support synchronous cancellation through `Drop::drop`. By accepting a `Cancel` as
/// input, a future expresses the ability to terminate early (though not immediately) after
/// `Cancel::cancel` has been called.
impl Cancel {
    pub fn new() -> Self {
        Cancel(Arc::new(CancelInner {
            semaphore: Semaphore::new(0),
            listeners: Mutex::new(vec![]),
        }))
    }

    pub fn cancel(&self) {
        self.0.semaphore.poison();
        for listener in self.0.listeners.lock().unwrap().drain(..) {
            listener();
        }
    }

    /// Wait for cancel to be called.
    pub async fn wait(&self) {
        self.0.semaphore.acquire(1).await.unwrap_err();
    }

    pub fn on_cancel(&self, listener: impl 'static + Send + FnOnce()) {
        self.0.listeners.lock().unwrap().push(Box::new(listener));
    }

    pub fn attach(&self, parent: &Self) {
        let this = self.clone();
        parent.on_cancel(move || this.cancel());
    }

    // Run the f until cancel is called, then drop f. This effectively wraps a synchronously
    // canceled future as an asynchronously canceled future.
    pub async fn checked<F: Future>(&self, success: F) -> Result<F::Output, Canceled> {
        select!(
            biased;
            _ = self.wait() => return Err(Canceled),
            x = success => return Ok(x),
        )
    }

    // Run f until a timeout after cancellation. This effectively puts a timeout on asynchronous
    // cancellation and uses synchronous cancellation after the timeout.
    pub async fn checked_timeout<F: Future>(&self, duration: Duration, fut: F) -> Result<F::Output, Timeout> {
        let failure = async {
            self.wait().await;
            sleep(duration).await;
        };
        select! {
            biased;
            _ = failure => Err(Timeout),
            output = fut => Ok(output),
        }
    }

    // Spawn a remote task governed by this Cancel. Dropping the returned future will trigger cancel.
    // This effectively wraps an asynchronously canceled future as a synchronously canceled future.
    pub fn spawn<F, S>(
        &self,
        spawn: &S,
        fut: F,
    ) -> CoopJoinHandle<S::JoinHandle<F::Output>>
        where F: Future + Send + 'static,
              F::Output: Send,
              S: Spawn {
        CoopJoinHandle { inner: spawn.spawn_with_handle(fut), cancel: self.clone() }
    }

    pub fn cancel_on_control_c(&self) {
        let counter = AtomicUsize::new(0);
        let this = self.clone();
        let prefix = "\nReceived Ctrl-C";
        ctrlc::set_handler(move || {
            match counter.fetch_add(1, Ordering::SeqCst) {
                0 => {
                    eprintln!("{}: cancelling.", prefix);
                    this.clone().cancel();
                }
                1 => eprintln!("{}: skip cancellation?", prefix),
                2 => eprintln!("{}: skip cancellation??", prefix),
                3 => {
                    eprintln!("{}: exiting.", prefix);
                    exit(3)
                }
                4 => eprintln!("{}: skip exit handlers?", prefix),
                5 => eprintln!("{}: skip exit handlers??", prefix),
                _ => {
                    eprintln!("{}: aborting.", prefix);
                    abort()
                }
            }
        }).unwrap();
    }

    pub async fn run_main<E: Display>(&self, dur: Duration, f: impl Future<Output=Result<(), E>>) -> ! {
        self.cancel_on_control_c();
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
    }
}

impl<J: JoinHandle> Future for CoopJoinHandle<J> {
    type Output = J::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx)
    }
}

impl<J: JoinHandle> JoinHandle for CoopJoinHandle<J> {
    fn abort(self) {
        self.cancel.cancel();
    }
}

impl From<Canceled> for io::Error {
    fn from(x: Canceled) -> Self { io::Error::new(ErrorKind::Interrupted, x) }
}

impl From<Timeout> for io::Error {
    fn from(x: Timeout) -> Self { io::Error::new(ErrorKind::TimedOut, x) }
}

impl From<oneshot::error::RecvError> for Canceled {
    fn from(_: oneshot::error::RecvError) -> Self {
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


#[tokio::test]
async fn test_return() {
    let cancel = Cancel::new();
    let handle = cancel.spawn(&Handle::current(), {
        async move {
            println!("Returning on {:?}", thread::current());
            1
        }
    });
    pin!(handle);
    println!("Joining on {:?}", thread::current());
    assert!(handle.as_mut().ready().is_none());
    yield_now().await;
    assert_eq!(1, handle.ready().unwrap());
}

impl Debug for Cancel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Cancel").field(&Arc::as_ptr(&self.0)).finish()
    }
}

#[tokio::test]
async fn test_abort_handle() {
    use std::iter;

    let cancel = Cancel::new();
    let (sender, mut receiver) = channel(3);
    let handle = cancel.spawn(&Handle::current(), {
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
    yield_now().await;
    poll_fn(|cx| {
        assert_eq!(Poll::Ready(Some(0)), receiver.poll_recv(cx));
        assert_eq!(Poll::Ready(Some(1)), receiver.poll_recv(cx));
        assert_eq!(Poll::Ready(Some(2)), receiver.poll_recv(cx));
        assert_eq!(Poll::Pending, receiver.poll_recv(cx));
        Poll::Ready(())
    }).await;
    handle.abort();
    poll_fn(|cx| {
        assert_eq!(Poll::Pending, receiver.poll_recv(cx));
        Poll::Ready(())
    }).await;
    yield_now().await;
    poll_fn(|cx| {
        assert_eq!(Poll::Ready(Some(-1)), receiver.poll_recv(cx));
        assert_eq!(Poll::Ready(None), receiver.poll_recv(cx));
        Poll::Ready(())
    }).await;
}