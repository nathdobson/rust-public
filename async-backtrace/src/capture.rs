use std::{mem, slice, fmt};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::{Write, Display, Formatter};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::ptr::null_mut;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::Duration;

use backtrace::{Backtrace, BacktraceFrame, BacktraceSymbol, SymbolName};
use futures_util::task::noop_waker;
use itertools::Itertools;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use tokio::task::JoinHandle;

use termio::color::Color;
use termio::output::{Background, Foreground};
use util::weak_vec::WeakVec;

use crate::remangle::remangle;
use futures_util::future::FusedFuture;
use futures_util::FutureExt;

lazy_static! {
    static ref TASKS: Mutex<WeakVec<Mutex<Task<dyn Send+FusedFuture<Output=()>>>>> = Mutex::new(WeakVec::new());
}

const INDENT_CONTINUE: &'static str /**/ = "┃   ";
const INDENT_TEE: &'static str /*     */ = "┣━━━";
const INDENT_END: &'static str /*     */ = "┗━━━";
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
            write!(output, "{}{} task(s)\n", indent, self.count);
            //write!(output, "{}\n", indent);
        }
        let old_indent = indent.len();
        for (index, (addr, child)) in self.children.iter().enumerate() {
            let mut symbols = vec![];
            backtrace::resolve(*addr, |symbol| {
                let mut line = String::new();
                if let Some(name) = symbol.name() {
                    write!(&mut line, "{}", remangle(&name.to_string()));
                    if let Some(filename) = symbol.filename() {
                        let filename =
                            filename.to_str().unwrap()
                                .split("src/").last().unwrap()
                                .split("examples/").last().unwrap();
                        if let Some(lineno) = symbol.lineno() {
                            if let Some(colno) = symbol.colno() {
                                write!(&mut line, " ({}:{}:{})", filename, lineno, colno);
                            } else {
                                write!(&mut line, " ({}:{})", filename, lineno);
                            }
                        } else {
                            write!(&mut line, " ({})", filename);
                        }
                    }
                }
                symbols.push(line);
            });
            for (depth, symbol) in symbols.iter().enumerate() {
                let extend = depth == 0 && index != self.children.len() - 1;
                let space = depth == 0 && self.children.len() > 1;
                let old_indent = indent.len();
                let mut extra_indent = "";
                if extend {
                    write!(output, "{}{}\n", INDENT_CONTINUE, indent);
                    extra_indent = INDENT_TEE;
                } else if space {
                    write!(output, "{}{}\n", INDENT_CONTINUE, indent);
                    extra_indent = INDENT_END;
                }
                write!(output, "{}{}{}\n", indent, extra_indent, symbol)?;
                if extend {
                    indent.push_str(INDENT_CONTINUE);
                } else if space {
                    indent.push_str(INDENT_BLANK);
                }
            }
            child.print(indent, output);
            indent.truncate(old_indent);
        }
        indent.truncate(old_indent);
        Ok(())
    }
}


impl<'a> TraceWaker<'a> {
    fn clone_impl(&self) -> Waker {
        let backtrace = Backtrace::new_unresolved();
        let slice = backtrace.frames();
        let mut lock = self.trace.lock();
        if slice.len() >= lock.ignore_prefix + lock.ignore_suffix {
            let slice = &slice[lock.ignore_prefix..slice.len() - lock.ignore_suffix];
            lock.node.insert(slice.iter());
        }
        mem::drop(lock);
        self.internal.clone()
    }
    fn wake_by_ref_impl(&self) {
        self.internal.wake_by_ref();
    }
}

static TRACE_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |x| unsafe { mem::transmute((*(x as *const TraceWaker)).clone_impl()) },
    |_| panic!(),
    |x| unsafe { (*(x as *const TraceWaker)).wake_by_ref_impl() },
    |_| (),
);

impl Trace {
    #[inline(never)]
    pub fn new() -> Self {
        unsafe {
            let tasks: Vec<_> = TASKS.lock().iter().collect();
            let trace = backtrace::Backtrace::new();
            let split = trace.frames().iter().position(|frame| {
                frame.symbols().iter().any(|symbol: &BacktraceSymbol| {
                    symbol.name().iter().any(|name: &SymbolName|
                        name.to_string().starts_with("async_backtrace::Trace::new")
                    )
                })
            });
            let ignore_prefix = split.unwrap_or(0);
            let ignore_suffix = split.map_or(0, |n| trace.frames().len() - n);
            let trace = Mutex::new(Trace { timeouts: 0, ignore_prefix, ignore_suffix, node: Node::new() });
            for task in tasks {
                if let Some(mut lock) = task.try_lock_for(Duration::from_millis(100)) {
                    let waker = TraceWaker { internal: &lock.waker, trace: &trace };
                    let waker = Waker::from_raw(RawWaker::new(&waker as *const TraceWaker as *const (), &TRACE_WAKER_VTABLE));
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