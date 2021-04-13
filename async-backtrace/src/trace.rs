use std::{mem, slice, fmt, thread};
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
use std::lazy::OnceCell;
use util::weak_vec::WeakVec;

use crate::remangle;
use util::shared::ObjectInner;
use std::hint::black_box;
use async_util::waker::{noop_waker, noop_waker_ref};
use async_util::fused::Fused;
use async_util::futureext::FutureExt;
use std::marker::PhantomData;


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
pub struct TraceInner {
    traced_delimiter: bool,
    ignore_prefix: usize,
    ignore_suffix: usize,
    node: Node,
}

pub struct Trace(Mutex<TraceInner>);

pub struct TraceWakerInner<'a> {
    internal: &'a Waker,
    trace: &'a Trace,
}

pub struct TraceWaker<'a> {
    waker: Waker,
    phantom: PhantomData<&'a TraceWakerInner<'a>>,
}

const INDENT_BLANK: &'static str /*   */ = "    ";
const INDENT_END: &'static str /*     */ = "┏━━ ";
const INDENT_CONTINUE: &'static str /**/ = "┃   ";
const INDENT_TEE: &'static str /*     */ = "┣━━ ";

impl Trace {
    pub fn new() -> Trace {
        Trace(Mutex::new(TraceInner {
            traced_delimiter: false,
            ignore_prefix: 0,
            ignore_suffix: 0,
            node: Node::new(),
        }))
    }
    pub fn with_internal<'a>(&'a self, internal: &'a Waker) -> TraceWakerInner<'a> {
        TraceWakerInner { internal, trace: self }
    }
}

impl<'a> TraceWakerInner<'a> {
    pub fn as_waker(&'a self) -> TraceWaker<'a> {
        unsafe {
            TraceWaker {
                waker: Waker::from_raw(
                    RawWaker::new(self as *const TraceWakerInner as *const (),
                                  &TRACE_WAKER_VTABLE)),
                phantom: PhantomData,
            }
        }
    }
}

impl<'a> TraceWaker<'a> {
    pub fn as_context(&'a self) -> Context<'a> {
        Context::from_waker(&self.waker)
    }
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

impl<'a> TraceWakerInner<'a> {
    fn trace(&self) {
        let backtrace = Backtrace::new_unresolved();
        let slice = backtrace.frames();
        let mut lock = self.trace.0.lock();
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
}

static TRACE_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |x| unsafe { mem::transmute((*(x as *const TraceWakerInner)).clone_impl()) },
    |_| panic!(),
    |x| unsafe { (*(x as *const TraceWakerInner)).wake_by_ref_impl() },
    |_| (),
);

#[inline(never)]
pub fn delimiter(cx: &mut Context) {
    mem::drop(cx.waker().clone());
}

static DELIMITER_FN: fn(&mut Context) = delimiter as fn(&mut Context);

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

impl Display for Trace {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        NodePrinter::new(f).print(&self.0.lock().node)?;
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
    use crate::trace::TraceGroup;
    use crate::{spawn, TraceGroup};

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

    async fn test_basic() {
        let group = TraceGroup::new();
        tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().on_thread_start(group.on_thread_start()).build().unwrap().block_on(async {
            spawn(async move {
                join!(foo4(), bar(), sleep(Duration::from_millis(1000)));
            });
            spawn(baz());
            sleep(Duration::from_millis(100)).await;
            println!("{}", group.capture());
        });
    }
}