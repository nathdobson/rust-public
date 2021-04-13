use std::future::Future;
use tokio::task::JoinHandle;
use async_util::spawn::Spawn;
use async_util::futureext::FutureExt;
use async_util::join::RemoteJoinHandle;
use async_util::priority::{PriorityPool, Priority, PrioritySpawn};
use crate::TraceGroup;

pub fn spawn<F: 'static + Send + Future<Output=()>>(fut: F) -> JoinHandle<()> {
    tokio::spawn(TraceGroup::current().push(fut))
}

pub struct TracedSpawn<S: Spawn> {
    group: TraceGroup,
    inner: S,
}

impl<S: Spawn> Spawn for TracedSpawn<S> {
    type JoinHandle<T: 'static + Send> = RemoteJoinHandle<T>;

    fn spawn_with_handle<F: 'static + Send + Future>(&self, fut: F) -> Self::JoinHandle<F::Output> where F::Output: 'static + Send {
        let (remote, handle) = fut.into_remote();
        self.inner.spawn(self.group.push(remote));
        handle
    }

    fn spawn<F: 'static + Send + Future<Output=()>>(&self, fut: F) {
        self.inner.spawn(self.group.push(fut));
    }
}

#[derive(Clone)]
pub struct TracedPriorityPool<P: Priority> {
    group: TraceGroup,
    inner: PriorityPool<P>,
}

impl<P: Priority> TracedPriorityPool<P> {
    pub fn new(inner: PriorityPool<P>) -> Self {
        TracedPriorityPool { group: TraceGroup::current(), inner }
    }
    pub fn with_priority(&self, priority: P) -> TracedSpawn<PrioritySpawn<P>> {
        TracedSpawn { group: self.group.clone(), inner: self.inner.with_priority(priority) }
    }
}

