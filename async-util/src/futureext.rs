use std::future::Future;
use std::task::{Poll, Wake, Context};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use tokio::pin;
use crate::join::{RemoteJoinHandle, Remote, remote};
use std::pin::Pin;
use crate::fused::Fused;

pub trait FutureExt: Future {
    fn ready(self) -> Option<Self::Output> where Self: Sized {
        struct Woken(AtomicBool);
        impl Wake for Woken {
            fn wake(self: Arc<Self>) {
                self.0.store(true, SeqCst);
            }
        }
        let woken = Arc::new(Woken(AtomicBool::new(true)));
        let waker = woken.clone().into();
        let mut cx = Context::from_waker(&waker);
        let fut = self;
        pin!(fut);
        while woken.0.swap(false, SeqCst) {
            if let Poll::Ready(result) = fut.as_mut().poll(&mut cx) {
                return Some(result);
            }
        }
        return None;
    }
    fn into_remote(self) -> (Remote<Self>, RemoteJoinHandle<Self::Output>) where Self: Sized {
        remote(self)
    }
    fn boxed<'a>(self) -> Pin<Box<dyn 'a + Send + Future<Output=Self::Output>>> where Self: Sized + Send + 'a {
        Box::pin(self)
    }
    fn fuse(self) -> Fused<Self> where Self: Sized, Self: Future<Output=()> {
        Fused::new(self)
    }
}

impl<T: Future> FutureExt for T {}