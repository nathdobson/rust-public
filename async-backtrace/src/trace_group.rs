use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use async_util::waker::{noop_waker_ref, noop_waker};
use parking_lot::Mutex;
use crate::Trace;
use std::time::{Instant, Duration};
use std::{thread, mem};
use util::weak_vec::WeakVec;
use async_util::futureext::FutureExt;
use async_util::fused::Fused;
use std::lazy::OnceCell;
use backtrace::trace;
use crate::trace::delimiter;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use pin_project::pin_project;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::Stream;
use async_util::promise::Promise;
use async_util::promise;
use tokio::time::timeout;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

#[pin_project]
pub struct TraceFut<F: Future<Output=()>> {
    sender: Arc<mpsc::Sender<TraceRequest>>,
    receiver: mpsc::Receiver<TraceRequest>,
    #[pin]
    inner: F,
}

thread_local! {
    pub static CURRENT_TRACE_GROUP: OnceCell<TraceGroup> = OnceCell::new();
}

#[derive(Clone)]
struct TraceRequest {
    trace: Trace,
    completed: Promise<!>,
}

#[derive(Clone)]
pub struct TraceGroup(Arc<Mutex<WeakVec<mpsc::Sender<TraceRequest>>>>);

impl TraceGroup {
    pub fn new() -> Self {
        TraceGroup(Arc::new(Mutex::new(WeakVec::new())))
    }
    pub fn push<F: Future<Output=()>>(&self, fut: F) -> TraceFut<F> {
        let (sender, receiver) = mpsc::channel(1);
        let sender = Arc::new(sender);
        self.0.lock().push(Arc::downgrade(&sender));
        TraceFut { sender, receiver, inner: fut }
    }
    pub fn on_thread_start(&self) -> impl Fn() + Send + Sync + 'static {
        let this = self.clone();
        move || this.clone().set_current()
    }
    pub fn set_current(self) {
        CURRENT_TRACE_GROUP.with(move |current_trace_group| {
            current_trace_group.set(self).ok().unwrap()
        });
    }
    pub fn current() -> Self {
        CURRENT_TRACE_GROUP.with(|current_trace_group| current_trace_group.get().unwrap().clone())
    }
}

impl<F: Future<Output=()>> TraceFut<F> {}

impl<F: Future<Output=()>> Future for TraceFut<F> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let mut project = self.project();
        let mut polled_inner = false;
        loop {
            match Pin::new(&mut *project.receiver).poll_recv(cx) {
                Poll::Pending => break,
                Poll::Ready(None) => break,
                Poll::Ready(Some(request)) => {
                    polled_inner = true;
                    let result = project.inner.as_mut().poll(&mut request.trace.with_internal(&cx.waker()).as_waker().as_context());
                    if result.is_ready() {
                        return result;
                    }
                }
            }
        }
        if polled_inner {
            return Poll::Pending;
        }
        project.inner.as_mut().poll(cx)
    }
}

impl TraceGroup {
    #[inline(never)]
    pub async fn capture(&self) -> Trace {
        let start = Instant::now();
        let trace = Trace::new();
        let tasks: Vec<_> = self.0.lock().iter().collect();
        for task in tasks {
            let completed = Promise::new();
            task.send(TraceRequest { trace: trace.clone(), completed: completed.clone() }).await.ok();
            let receiver = completed.receiver();
            mem::drop(completed);
            if let Err(e) = timeout(Duration::from_millis(100), receiver.recv_none()).await {
                eprintln!("Timeout while capturing");
            }
        }
        trace
    }
}
