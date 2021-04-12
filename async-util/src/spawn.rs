use std::future::Future;
use std::any::Any;
use std::thread;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::panic::resume_unwind;
use crate::join::JoinHandle;

pub trait Spawn {
    type JoinHandle<T: 'static + Send>: 'static + Send + JoinHandle<Output=T>;
    fn spawn_with_handle<F: 'static + Send + Future>(&self, fut: F) -> Self::JoinHandle<F::Output> where F::Output: 'static + Send;
    fn spawn<F: 'static + Send + Future<Output=()>>(&self, fut: F);
}

pub struct TokioJoinHandle<T>(tokio::task::JoinHandle<T>);

impl<T> JoinHandle for TokioJoinHandle<T> {
    fn abort(self) { self.0.abort(); }
}

impl<T> Future for TokioJoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx).map(|r| r.unwrap_or_else(|e: tokio::task::JoinError| resume_unwind(e.into_panic())))
    }
}

impl Spawn for tokio::runtime::Handle {
    type JoinHandle<T: 'static + Send> = TokioJoinHandle<T>;

    fn spawn_with_handle<F: 'static + Send + Future>(&self, fut: F) -> TokioJoinHandle<F::Output> where F::Output: 'static + Send {
        TokioJoinHandle(self.spawn(fut))
    }

    fn spawn<F: 'static + Send + Future<Output=()>>(&self, fut: F) {
        tokio::spawn(fut);
    }
}