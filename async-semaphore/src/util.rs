use std::task::{Poll, Context, Waker};
use std::pin::Pin;
use std::future::Future;

pub unsafe fn assert_sync_send<F: Future>(future: F) -> impl Future<Output=F::Output> + Sync + Send {
    struct AssertSyncSend<F: Future>(F);
    impl<F: Future> Future for AssertSyncSend<F> {
        type Output = F::Output;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().0) }.poll(cx)
        }
    }
    unsafe impl<F: Future> Send for AssertSyncSend<F> {}
    unsafe impl<F: Future> Sync for AssertSyncSend<F> {}

    AssertSyncSend(future)
}


pub fn yield_once<F: FnOnce()>(on_cancel: F) -> impl Future<Output=()> {
    struct YieldOnce<F: FnOnce()> {
        ready: bool,
        on_cancel: Option<F>,
    }
    impl<F: FnOnce()> Future for YieldOnce<F> {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
            unsafe {
                let this = self.get_unchecked_mut();
                if this.ready {
                    this.on_cancel = None;
                    Poll::Ready(())
                } else {
                    this.ready = true;
                    Poll::Pending
                }
            }
        }
    }
    impl<F: FnOnce()> Drop for YieldOnce<F> {
        fn drop(&mut self) {
            if let Some(on_cancel) = self.on_cancel.take() {
                on_cancel();
            }
        }
    }
    YieldOnce {
        ready: false,
        on_cancel: Some(on_cancel),
    }
}

struct CloneWaker;

impl Future for CloneWaker {
    type Output = Waker;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(cx.waker().clone())
    }
}

pub fn clone_waker() -> impl Future<Output=Waker> {
    CloneWaker
}
