use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project::pin_project;

#[pin_project]
pub struct Fused<F: Future<Output = ()>> {
    done: bool,
    #[pin]
    inner: F,
}

impl<F: Future<Output = ()>> Fused<F> {
    pub fn new(fut: F) -> Self
    where
        F: Sized,
    {
        Fused {
            done: false,
            inner: fut,
        }
    }
}

impl<F: Future<Output = ()>> Future for Fused<F> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let project = self.project();
        if *project.done {
            Poll::Ready(())
        } else {
            match project.inner.poll(cx) {
                Poll::Ready(output) => {
                    *project.done = true;
                    Poll::Ready(())
                }
                Poll::Pending => Poll::Pending,
            }
        }
    }
}
