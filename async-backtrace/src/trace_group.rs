use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use async_util::waker::{noop_waker_ref, noop_waker};
use parking_lot::Mutex;
use crate::Trace;
use std::time::{Instant, Duration};
use std::thread;
use util::weak_vec::WeakVec;
use async_util::futureext::FutureExt;
use async_util::fused::Fused;
use std::lazy::OnceCell;
use backtrace::trace;
use crate::trace::delimiter;
use std::sync::Arc;

pub trait TaskFuture = 'static + Send + Future<Output=()>;

struct Task<F: ?Sized + TaskFuture = dyn TaskFuture> {
    waker: Waker,
    fut: F,
}

pub struct TraceFut<F: TaskFuture> {
    task: Arc<Mutex<Task<Fused<F>>>>,
}

thread_local! {
    pub static CURRENT_TRACE_GROUP: OnceCell<TraceGroup> = OnceCell::new();
}

#[derive(Clone)]
pub struct TraceGroup(Arc<Mutex<WeakVec<Mutex<Task>>>>);

impl TraceGroup {
    pub fn new() -> Self { TraceGroup(Arc::new(Mutex::new(WeakVec::new()))) }
    pub fn push<F: TaskFuture>(&self, fut: F) -> TraceFut<F> {
        let task = Arc::new(Mutex::new(Task { waker: noop_waker(), fut: fut.fuse() }));
        self.0.lock().push(Arc::downgrade(&(task.clone() as Arc<Mutex<Task>>)));
        TraceFut { task }
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

impl<F: TaskFuture> TraceFut<F> {}

impl<F: TaskFuture> Future for TraceFut<F> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        unsafe {
            let this = self.get_unchecked_mut();
            let mut lock = this.task.lock();
            lock.waker = cx.waker().clone();
            Pin::new_unchecked(&mut lock.fut).poll(cx)
        }
    }
}

impl<F: TaskFuture> Unpin for TraceFut<F> {}

impl TraceGroup {
    #[inline(never)]
    pub fn capture(&self) -> Trace {
        unsafe {
            let start = Instant::now();
            let tasks: Vec<_> = self.0.lock().iter().collect();
            let trace = Trace::new();
            delimiter(&mut trace.with_internal(noop_waker_ref()).as_waker().as_context());
            for task in tasks {
                if let Some(mut lock) = task.try_lock_for(Duration::from_millis(100)) {
                    let lock = &mut *lock;
                    let pin = Pin::new_unchecked(&mut lock.fut);
                    pin.poll(&mut trace.with_internal(&lock.waker).as_waker().as_context()).is_ready();
                } else {
                    eprintln!("Timeout while tracing");
                }
            }
            trace
        }
    }
}
