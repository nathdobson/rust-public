use std::future::Future;
use std::task::{Context, Poll};
use std::pin::Pin;
use pin_project::pin_project;

#[pin_project]
pub struct Fused<F: Future<Output=()>> {
    #[pin]
    inner: Option<F>,
}

impl<F: Future<Output=()>> Fused<F> {
    pub fn new(fut: F) -> Self where F: Sized {
        Fused { inner: Some(fut) }
    }
}

impl<F: Future<Output=()>> Future for Fused<F> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(inner) = self.project().inner.as_pin_mut() {
            match inner.poll(cx) {
                Poll::Ready(output) => Poll::Ready(()),
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(())
        }
    }
}