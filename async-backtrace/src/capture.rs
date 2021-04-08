use std::{mem, slice, fmt};
use std::collections::hash_map::{DefaultHasher, Iter};
use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::{Write, Display, Formatter};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::ptr::null_mut;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::{Duration, Instant};
use crate::remangle::resolve_remangle;
use backtrace::{Backtrace, BacktraceFrame, BacktraceSymbol, SymbolName, resolve};
use futures_util::task::{noop_waker, noop_waker_ref};
use itertools::{Itertools, ExactlyOneError};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use tokio::task::JoinHandle;

use termio::color::Color;
use termio::output::{Background, Foreground};
use util::weak_vec::WeakVec;

use crate::remangle::remangle;
use futures_util::future::FusedFuture;
use futures_util::FutureExt;
use util::shared::ObjectInner;
use std::hint::black_box;

lazy_static! {
    static ref TASKS: Mutex<WeakVec<Mutex<Task<dyn Send+FusedFuture<Output=()>>>>> = Mutex::new(WeakVec::new());
}

const INDENT_CONTINUE: &'static str /**/ = "┃   ";
const INDENT_TEE: &'static str /*     */ = "┣━━ ";
const INDENT_END: &'static str /*     */ = "┗━━ ";
const INDENT_BLANK: &'static str /*   */ = "    ";

struct Task<F: ?Sized + 'static + Send + FusedFuture<Output=()>> {
    waker: Waker,
    fut: F,
}

pub struct Tracer<F: 'static + Send + FusedFuture<Output=()>> {
    task: Arc<Mutex<Task<F>>>,
}

impl<F: 'static + Send + FusedFuture<Output=()>> Tracer<F> {
    fn new(fut: F) -> Self {
        let task = Arc::new(Mutex::new(Task { waker: noop_waker(), fut }));
        TASKS.lock().push(Arc::downgrade(&(task.clone() as Arc<Mutex<Task<dyn Send + FusedFuture<Output=()>>>>)));
        Tracer { task }
    }
}

impl<F: 'static + Send + FusedFuture<Output=()>> Future for Tracer<F> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        unsafe {
            let this = self.get_unchecked_mut();
            let mut lock = this.task.lock();
            lock.waker = cx.waker().clone();
            let pin = Pin::new_unchecked(&mut lock.fut);
            if pin.is_terminated() {
                return Poll::Ready(());
            }
            pin.poll(cx)
        }
    }
}


impl<F: 'static + Send + FusedFuture<Output=()>> Unpin for Tracer<F> {}

pub fn spawn<F: 'static + Send + Future<Output=()>>(x: F) -> JoinHandle<()> {
    tokio::spawn(Tracer::new(x.fuse()))
}

#[derive(Debug)]
struct Node {
    children: HashMap<*mut c_void, Node>,
    count: usize,
}

#[derive(Debug)]
pub struct Trace {
    tasks: usize,
    timeouts: usize,
    ignore_prefix: usize,
    ignore_suffix: usize,
    node: Node,
}

struct TraceWaker<'a> {
    internal: &'a Waker,
    trace: &'a Mutex<Trace>,
}

impl Node {
    fn new() -> Self {
        Node { children: HashMap::new(), count: 0 }
    }
    fn insert(&mut self, mut frames: slice::Iter<BacktraceFrame>) {
        if let Some(next) = frames.next_back() {
            self.children.entry(next.ip()).or_insert(Node::new()).insert(frames);
        } else {
            self.count += 1;
        }
    }
    fn print(&self, indent: &mut String, output: &mut Formatter<'_>) -> fmt::Result {
        if self.count > 0 {
            write!(output, "{}{} pending\n", indent, self.count)?;
            //write!(output, "{}\n", indent);
        }
        let old_indent = indent.len();
        for (index, (addr, child)) in self.children.iter().enumerate() {
            let symbols = resolve_remangle(*addr);
            for (depth, symbol) in symbols.iter().enumerate() {
                let extend = depth == 0 && index != self.children.len() - 1;
                let space = depth == 0 && self.children.len() > 1;
                let old_indent = indent.len();
                let mut extra_indent = "";
                if extend {
                    write!(output, "{}{}\n", indent, INDENT_CONTINUE)?;
                    extra_indent = INDENT_TEE;
                } else if space {
                    write!(output, "{}{}\n", indent, INDENT_CONTINUE)?;
                    extra_indent = INDENT_END;
                }
                write!(output, "{}{}{}\n", indent, extra_indent, symbol)?;
                if extend {
                    indent.push_str(INDENT_CONTINUE);
                } else if space {
                    indent.push_str(INDENT_BLANK);
                }
            }
            child.print(indent, output)?;
            indent.truncate(old_indent);
        }
        indent.truncate(old_indent);
        Ok(())
    }
}


impl<'a> TraceWaker<'a> {
    fn trace(&self) {
        let backtrace = Backtrace::new_unresolved();
        let slice = backtrace.frames();
        let mut lock = self.trace.lock();
        if slice.len() >= lock.ignore_prefix + lock.ignore_suffix {
            let slice = &slice[lock.ignore_prefix..slice.len() - lock.ignore_suffix];
            lock.node.insert(slice.iter());
        }
        mem::drop(lock);
    }
    fn clone_impl(&self) -> Waker {
        self.trace();
        self.internal.clone()
    }
    fn wake_by_ref_impl(&self) {
        self.trace();
        self.internal.wake_by_ref();
    }
    fn new(internal: &'a Waker, trace: &'a Mutex<Trace>) -> Self {
        TraceWaker { internal: &internal, trace: &trace }
    }
    unsafe fn as_waker(&self) -> Waker {
        Waker::from_raw(RawWaker::new(self as *const TraceWaker as *const (), &TRACE_WAKER_VTABLE))
    }
}

static TRACE_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |x| unsafe { mem::transmute((*(x as *const TraceWaker)).clone_impl()) },
    |_| panic!(),
    |x| unsafe { (*(x as *const TraceWaker)).wake_by_ref_impl() },
    |_| (),
);

#[inline(never)]
fn poll_split_point(waker: &Waker) {
    mem::drop(waker.clone());
}

impl Trace {
    #[inline(never)]
    pub fn new() -> Self {
        unsafe {
            let start = Instant::now();
            let tasks: Vec<_> = TASKS.lock().iter().collect();
            // let trace = backtrace::Backtrace::new();
            // let split = trace.frames().iter().position(|frame| {
            //     frame.symbols().iter().any(|symbol: &BacktraceSymbol| {
            //         symbol.name().iter().any(|name: &SymbolName| {
            //             let name = name.to_string();
            //             name.starts_with("async_backtrace::Trace::new")
            //                 || (
            //                 name.starts_with("<async_backtrace[")
            //                     && name.ends_with("]::capture::Trace>::new")
            //             )
            //         })
            //     })
            // });
            let trace = Mutex::new(Trace {
                tasks: tasks.len(),
                timeouts: 0,
                ignore_prefix: 0,
                ignore_suffix: 0,
                node: Node::new(),
            });
            let no_waker = TraceWaker::new(noop_waker_ref(), &trace);
            let no_waker = no_waker.as_waker();
            let poll_split_point_fn = black_box(poll_split_point as fn(&Waker));
            let mut poll_split_point_symbol = None;
            resolve((poll_split_point_fn as usize + 1) as *mut c_void, |symbol| {
                if poll_split_point_symbol == None {
                    poll_split_point_symbol =
                        symbol.name()
                            .and_then(|name| name.as_str())
                            .map(|name| name.to_string());
                }
            });
            poll_split_point_fn(&no_waker);
            {
                let mut lock = trace.lock();
                if let Some(poll_split_point_symbol) = poll_split_point_symbol {
                    let mut node = &lock.node;
                    let mut index = 0;
                    let mut suffix = None;
                    loop {
                        let mut children = node.children.iter();
                        if let Some((addr, child)) = children.next() {
                            resolve(*addr, |symbol| {
                                if let Some(name) = symbol.name() {
                                    if let Some(name) = name.as_str() {
                                        if poll_split_point_symbol == name {
                                            suffix = Some(index);
                                        }
                                    }
                                }
                            });
                            index += 1;
                            node = child;
                        } else {
                            break;
                        }
                        assert!(children.next().is_none());
                    }
                    if let Some(suffix) = suffix {
                        lock.ignore_prefix = index - suffix - 1;
                        lock.ignore_suffix = suffix;
                    }
                }
                lock.node.children.clear();
            }
            for task in tasks {
                if let Some(mut lock) = task.try_lock_for(Duration::from_millis(100)) {
                    let waker = TraceWaker::new(&lock.waker, &trace);
                    let waker = waker.as_waker();
                    let pin = Pin::new_unchecked(&mut lock.fut);
                    if !pin.is_terminated() {
                        pin.poll(&mut Context::from_waker(&waker)).is_ready();
                    }
                } else {
                    trace.lock().timeouts += 1;
                }
            }
            trace.into_inner()
        }
    }
}

impl Display for Trace {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} spawned task(s):\n", self.tasks)?;
        if self.timeouts > 0 {
            write!(f, "Tracing timed out for {:?} tasks.", self.timeouts)?;
        }
        self.node.print(&mut String::new(), f)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;
    use std::future::Future;
    use std::hint::black_box;
    use std::mem::size_of;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::thread;
    use std::time::Duration;

    use pin_project::pin_project;
    use tokio::join;
    use tokio::time::sleep;

    use crate::capture::{spawn, Trace};

    #[inline(never)]
    async fn foo1() {
        sleep(Duration::from_millis(1000)).await;
    }

    #[inline(never)]
    async fn foo2<T: Debug>(x: T) {
        foo1().await;
        foo1().await;
        println!("{:?}", x);
    }

    #[inline(never)]
    async fn foo3() {
        foo2::<usize>(1usize).await;
        foo2::<u8>(2u8).await;
    }

    #[inline(never)]
    fn foo4() -> impl Future<Output=()> {
        #[pin_project]
        struct Foo4<F: Future>(#[pin] F);
        impl<F: Future> Future for Foo4<F> {
            type Output = F::Output;
            #[inline(never)]
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                black_box(self.project().0.poll(cx))
            }
        }
        Foo4(foo3())
    }

    #[inline(never)]
    async fn bar() {
        join!(foo4(), foo4());
    }

    #[inline(never)]
    async fn baz() {
        thread::sleep(Duration::from_millis(1000));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_basic() {
        spawn(async move {
            join!(foo4(), bar(), sleep(Duration::from_millis(1000)));
        });
        spawn(baz());
        sleep(Duration::from_millis(100)).await;
        println!("{}", Trace::new());
    }
}