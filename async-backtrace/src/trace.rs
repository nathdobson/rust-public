use std::{mem, slice, fmt, thread};
use std::collections::hash_map::{DefaultHasher, Iter};
use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::{Write, Display, Formatter, Arguments};
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
use tokio::pin;


use crate::{remangle, SyncDisplay};
use util::shared::ObjectInner;
use std::hint::black_box;
use async_util::waker::{noop_waker, noop_waker_ref};
use async_util::fused::Fused;
use async_util::futureext::FutureExt;
use std::marker::PhantomData;
use std::future::poll_fn;
use std::cmp::Reverse;

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
enum NodeKey {
    Address(*mut c_void),
    Annotation(String),
}

#[derive(Debug)]
struct Node {
    children: HashMap<NodeKey, Node>,
    count: usize,
    messages: Vec<String>,
    recursive_size: usize,
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
    pub fn from_waker(waker: &'a Waker) -> Option<&'a TraceWakerInner<'a>> {
        unsafe {
            let (data, vtable): (*const (), &'static RawWakerVTable) = mem::transmute_copy(waker);
            if vtable as *const RawWakerVTable == &TRACE_WAKER_VTABLE {
                return Some(&*(data as *const TraceWakerInner));
            } else {
                None
            }
        }
    }
}

pub async fn trace_annotate<F: Future>(message: &dyn SyncDisplay, fut: F) -> F::Output {
    pin!(fut);
    poll_fn(|cx| {
        if let Some(trace_waker_inner) = TraceWakerInner::from_waker(cx.waker()) {
            if let Some(node) = trace_waker_inner.trace.0.lock().trace() {
                let node =
                    node.children
                        .entry(NodeKey::Annotation(format!("● {}", message)))
                        .or_insert(Node::new());
                let subtrace = Trace::new();
                delimiter(&mut subtrace.with_internal(noop_waker_ref()).as_waker().as_context());
                mem::swap(node, &mut subtrace.0.lock().node);
                let result = fut.as_mut().poll(&mut subtrace.with_internal(trace_waker_inner.internal).as_waker().as_context());
                mem::swap(node, &mut subtrace.0.lock().node);
                return result;
            }
        }
        fut.as_mut().poll(cx)
    }).await
}

impl<'a> TraceWaker<'a> {
    pub fn as_context(&'a self) -> Context<'a> {
        Context::from_waker(&self.waker)
    }
}

impl Node {
    fn new() -> Self {
        Node { children: HashMap::new(), count: 0, messages: vec![], recursive_size: 0 }
    }
    fn insert(&mut self, mut frames: slice::Iter<BacktraceFrame>) -> &mut Node {
        if let Some(next) = frames.next_back() {
            self.children.entry(NodeKey::Address(next.ip())).or_insert(Node::new()).insert(frames)
        } else {
            self
        }
    }
    fn compute_recursive_size(&mut self) {
        let mut total = 0;
        for child in self.children.values_mut() {
            child.compute_recursive_size();
            total += child.recursive_size + 1
        }
        self.recursive_size = total;
    }
}

impl<W: Write> NodePrinter<W> {
    fn new(writer: W) -> Self {
        NodePrinter { indent: "".to_string(), writer }
    }
    fn print(&mut self, node: &Node) -> fmt::Result {
        let old_indent = self.indent.len();
        let mut children: Vec<_> = node.children.iter().collect();
        children.sort_by_key(|(key, child)| (Reverse(child.recursive_size), *key));
        for (index, (addr, child)) in children.iter().enumerate() {
            let temp;
            let symbols = match addr {
                NodeKey::Address(addr) => resolve_remangle(*addr),
                NodeKey::Annotation(annotation) => {
                    temp = [annotation.as_str()];
                    &temp
                }
            };
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
        if node.count > 0 {
            writeln!(self.writer, "{}{} pending", self.indent, node.count)?;
        }
        for message in node.messages.iter() {
            writeln!(self.writer, "{}● {}", self.indent, message)?;
        }
        Ok(())
    }
}

impl TraceInner {
    fn trace(&mut self) -> Option<&mut Node> {
        let backtrace = Backtrace::new_unresolved();
        let slice = backtrace.frames();
        if !self.traced_delimiter {
            self.traced_delimiter = true;
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
                    self.ignore_prefix = prefix;
                    self.ignore_suffix = slice.len() - prefix - 1;
                }
            }
            self.node.children.clear();
            None
        } else {
            if slice.len() >= self.ignore_prefix + self.ignore_suffix {
                let slice = &slice[self.ignore_prefix..slice.len() - self.ignore_suffix];
                Some(self.node.insert(slice.iter()))
            } else {
                None
            }
        }
    }
}

impl<'a> TraceWakerInner<'a> {
    fn clone_impl(&self) -> Waker {
        self.trace.0.lock().trace().map(|x| x.count += 1);
        self.internal.clone()
    }
    fn wake_by_ref_impl(&self) {
        self.trace.0.lock().trace().map(|x| x.count += 1);
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
        self.0.lock().node.compute_recursive_size();
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
    use crate::{spawn, TraceGroup};
    use async_util::futureext::FutureExt;
    use crate::trace::trace_annotate;

    #[inline(never)]
    async fn foo1(msg: &str) {
        trace_annotate(&|f| write!(f, "foo1 {}", msg), sleep(Duration::from_millis(1000))).await;
    }

    #[inline(never)]
    async fn foo2<T: Debug>(x: T) {
        trace_annotate(&|f| write!(f, "foo2 custom message"), foo1("x")).await;
        foo1("y").await;
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
        trace_annotate(&|f| write!(f, "baz custom message"), async {
            thread::sleep(Duration::from_millis(1000))
        }).await;
    }

    #[test]
    fn test_basic() {
        let group = TraceGroup::new();
        let fut = {
            let group = group.clone();
            async move {
                spawn(async move {
                    join!(foo4(), bar(), sleep(Duration::from_millis(1000)));
                });
                spawn(foo1("a"));
                spawn(foo1("b"));
                spawn(baz());
                sleep(Duration::from_millis(100)).await;
                println!("{}", group.capture());
            }
        };
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().on_thread_start(group.on_thread_start()).build().unwrap();
        group.set_current();
        rt.block_on(fut)
    }
}