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
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker, Wake};
use std::time::{Duration, Instant};
use crate::remangle::resolve_remangle;
use backtrace::{Backtrace, BacktraceFrame, BacktraceSymbol, SymbolName, resolve};
use itertools::{Itertools, ExactlyOneError};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use tokio::task::JoinHandle;

use util::weak_vec::WeakVec;

use crate::remangle;
use util::shared::ObjectInner;
use std::hint::black_box;
use async_util::waker::{noop_waker, noop_waker_ref};
use async_util::fused::Fused;
use async_util::futureext::FutureExt;

pub trait TaskFuture = 'static + Send + Future<Output=()>;

struct Task<F: ?Sized + TaskFuture = dyn TaskFuture> {
    waker: Waker,
    done: bool,
    fut: F,
}

pub struct Tracer<F: TaskFuture> {
    task: Arc<Mutex<Task<Fused<F>>>>,
}

#[derive(Debug)]
struct Node {
    children: HashMap<*mut c_void, Node>,
    count: usize,
}

struct NodePrinter<W: Write> {
    indent: String,
    writer: W,
}

#[derive(Debug)]
pub struct Trace {
    tasks: usize,
    timeouts: usize,
    traced_delimiter: bool,
    ignore_prefix: usize,
    ignore_suffix: usize,
    node: Node,
}

struct TraceWaker<'a> {
    internal: &'a Waker,
    trace: &'a Mutex<Trace>,
}

lazy_static! {
    static ref TASKS: Mutex<WeakVec<Mutex<Task>>> = Mutex::new(WeakVec::new());
}

const INDENT_BLANK: &'static str /*   */ = "    ";
const INDENT_END: &'static str /*     */ = "┏━━ ";
const INDENT_CONTINUE: &'static str /**/ = "┃   ";
const INDENT_TEE: &'static str /*     */ = "┣━━ ";

impl<F: TaskFuture> Tracer<F> {
    fn new(fut: F) -> Self {
        let task = Arc::new(Mutex::new(Task { waker: noop_waker(), done: false, fut: fut.fuse() }));
        TASKS.lock().push(Arc::downgrade(&(task.clone() as Arc<Mutex<Task>>)));
        Tracer { task }
    }
}

impl<F: TaskFuture> Future for Tracer<F> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        unsafe {
            let this = self.get_unchecked_mut();
            let mut lock = this.task.lock();
            lock.waker = cx.waker().clone();
            if lock.done {
                return Poll::Ready(());
            }
            let pin = Pin::new_unchecked(&mut lock.fut);
            let result = pin.poll(cx);
            if result.is_ready() {
                lock.done = true;
            }
            result
        }
    }
}


impl<F: TaskFuture> Unpin for Tracer<F> {}

pub fn spawn<F: 'static + Send + Future<Output=()>>(x: F) -> JoinHandle<()> {
    tokio::spawn(Tracer::new(x))
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
}

impl<W: Write> NodePrinter<W> {
    fn new(writer: W) -> Self {
        NodePrinter { indent: "".to_string(), writer }
    }
    fn print(&mut self, node: &Node) -> fmt::Result {
        if node.count > 0 {
            write!(self.writer, "{}{} pending\n", self.indent, node.count)?;
        }
        let old_indent = self.indent.len();
        for (index, (addr, child)) in node.children.iter().enumerate() {
            let symbols = resolve_remangle(*addr);
            let extra = node.children.len() > 1;
            if extra {
                if index == 0 {
                    self.indent.push_str(INDENT_BLANK);
                } else {
                    self.indent.push_str(INDENT_CONTINUE);
                }
            }
            self.print(child)?;
            for (depth, symbol) in symbols.iter().enumerate().rev() {
                if extra && depth == 0 {
                    self.indent.truncate(old_indent);
                    if index == 0 {
                        self.indent.push_str(INDENT_END);
                    } else {
                        self.indent.push_str(INDENT_TEE);
                    }
                }
                writeln!(self.writer, "{}{}", self.indent, symbol)?;
                if extra && depth == 0 {
                    self.indent.truncate(old_indent);
                    self.indent.push_str(INDENT_CONTINUE);
                    writeln!(self.writer, "{}", self.indent)?;
                }
            }
            self.indent.truncate(old_indent);
        }
        self.indent.truncate(old_indent);
        Ok(())
    }
}

impl<'a> TraceWaker<'a> {
    fn trace(&self) {
        let backtrace = Backtrace::new_unresolved();
        let slice = backtrace.frames();
        let mut lock = self.trace.lock();
        if !lock.traced_delimiter {
            lock.traced_delimiter = true;
            if let Some(delimiter_name) = &*DELIMITER_NAME {
                let prefix = slice.iter().position(|symbol| {
                    let mut found = false;
                    resolve(symbol.symbol_address(), |symbol| {
                        if let Some(name) = symbol.name() {
                            if let Some(name) = name.as_str() {
                                if delimiter_name == name {
                                    found = true;
                                }
                            }
                        }
                    });
                    found
                });
                if let Some(prefix) = prefix {
                    lock.ignore_prefix = prefix;
                    lock.ignore_suffix = slice.len() - prefix - 1;
                }
            }
            lock.node.children.clear();
        } else {
            if slice.len() >= lock.ignore_prefix + lock.ignore_suffix {
                let slice = &slice[lock.ignore_prefix..slice.len() - lock.ignore_suffix];
                lock.node.insert(slice.iter());
            }
        }
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
fn delimiter(waker: &Waker) {
    mem::drop(waker.clone());
}

static DELIMITER_FN: fn(&Waker) = delimiter as fn(&Waker);

fn resolve_delimiter_name() -> Option<String> {
    let mut result = None;
    resolve((DELIMITER_FN as usize + 1) as *mut c_void, |symbol| {
        if result == None {
            result =
                symbol.name()
                    .and_then(|name| name.as_str())
                    .map(|name| name.to_string());
        }
    });
    result
}

lazy_static! {
    static ref DELIMITER_NAME: Option<String> = resolve_delimiter_name();
}

impl Trace {
    #[inline(never)]
    pub fn new() -> Self {
        unsafe {
            let start = Instant::now();
            let tasks: Vec<_> = TASKS.lock().iter().collect();
            let trace = Mutex::new(Trace {
                tasks: tasks.len(),
                timeouts: 0,
                traced_delimiter: false,
                ignore_prefix: 0,
                ignore_suffix: 0,
                node: Node::new(),
            });
            let no_waker = TraceWaker::new(noop_waker_ref(), &trace);
            let no_waker = no_waker.as_waker();
            delimiter(&no_waker);
            for task in tasks {
                if let Some(mut lock) = task.try_lock_for(Duration::from_millis(100)) {
                    let waker = TraceWaker::new(&lock.waker, &trace);
                    let waker = waker.as_waker();

                    let pin = Pin::new_unchecked(&mut lock.fut);
                    pin.poll(&mut Context::from_waker(&waker)).is_ready();
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
        writeln!(f, "{:?} spawned task(s):", self.tasks)?;
        if self.timeouts > 0 {
            writeln!(f, "Tracing timed out for {:?} tasks.", self.timeouts)?;
        }
        writeln!(f)?;
        NodePrinter::new(f).print(&self.node)?;
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

    use crate::async_capture::{spawn, Trace};

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