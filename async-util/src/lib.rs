#![feature(never_type)]

use async_std::sync;
use std::sync::Arc;
use async_std::channel;
use futures::{
    future::FutureExt,
    pin_mut,
    select,
};
use async_std::channel::RecvError;

struct CondvarInner {
    condvar: sync::Condvar,
    closed: channel::Receiver<!>,
}

#[derive(Clone)]
pub struct Condvar(Arc<CondvarInner>, channel::Sender<!>);

#[derive(Clone)]
pub struct CondvarWeak(Arc<CondvarInner>);

#[derive(Clone)]
pub struct Mutex(Arc<sync::Mutex<()>>);

pub struct MutexGuard<'a>(sync::MutexGuard<'a, ()>);

impl Mutex {
    pub fn new() -> Self {
        Mutex(Arc::new(sync::Mutex::new(())))
    }
    pub async fn lock<'a>(&'a self) -> MutexGuard<'a> {
        MutexGuard(self.0.lock().await)
    }
}

impl CondvarInner {
    async fn wait<'a>(&'a self, guard: MutexGuard<'a>) -> Result<MutexGuard<'a>, RecvError> {
        let wait = self.condvar.wait(guard.0).fuse();
        let recv = self.closed.recv().fuse();
        pin_mut!(wait, recv);
        select! {
            guard = wait => Ok(MutexGuard(guard)),
            _ = recv => Err(RecvError),
        }
    }
}

impl Condvar {
    pub fn new() -> Self {
        let (sender, receiver) = channel::unbounded();
        Condvar(Arc::new(CondvarInner {
            condvar: sync::Condvar::new(),
            closed: receiver,
        }), sender)
    }
    pub fn downgrade(&self) -> CondvarWeak {
        CondvarWeak(self.0.clone())
    }
    pub fn notify_one(&self) {
        self.0.condvar.notify_one();
    }
    pub fn notity_all(&self) {
        self.0.condvar.notify_all();
    }
    pub async fn wait<'a>(&'a self, guard: MutexGuard<'a>) -> Result<MutexGuard<'a>, RecvError> {
        self.0.wait(guard).await
    }
}

impl CondvarWeak {
    pub async fn wait<'a>(&'a self, guard: MutexGuard<'a>) -> Result<MutexGuard<'a>, RecvError> {
        self.0.wait(guard).await
    }
}
