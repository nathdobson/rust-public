use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::task::{Context, Poll, Wake};
use tokio::pin;

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
}

impl<T: ?Sized + Future> FutureExt for T {}