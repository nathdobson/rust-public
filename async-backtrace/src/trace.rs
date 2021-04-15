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
use lazy_static::{lazy_static, initialize};
use parking_lot::Mutex;
use tokio::task::JoinHandle;
use std::lazy::{OnceCell, SyncOnceCell};
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
    Address(usize),
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
    node: Node,
}

#[derive(Clone)]
pub struct Trace(Arc<Mutex<TraceInner>>);

pub enum Path<'a> {
    ConsAnnot {
        head: &'a dyn SyncDisplay,
        tail: &'a Path<'a>,
    },
    ConsStack {
        head: &'a [BacktraceFrame],
        tail: &'a Path<'a>,
    },
    Nil,
}

pub enum TraceWakerInner<'a> {
    Trace {
        internal: &'a Waker,
        trace: &'a Trace,
        path: Path<'a>,
        path_frames: usize,
    },
    Calibrate {
        result: &'a SyncOnceCell<Backtrace>,
    },
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
        initialize(&BACKTRACE_PREFIX);
        initialize(&WAKE_PREFIX);
        Trace(Arc::new(Mutex::new(TraceInner {
            node: Node::new(),
        })))
    }
    pub fn with_internal<'a>(&'a self, internal: &'a Waker) -> TraceWakerInner<'a> {
        TraceWakerInner::Trace {
            internal,
            trace: self,
            path: Path::Nil,
            path_frames: 0,
        }
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
        if let Some(TraceWakerInner::Trace { internal, trace, path, path_frames }) = TraceWakerInner::from_waker(cx.waker()) {
            let backtrace = Backtrace::new_unresolved();
            let end = backtrace.frames().len();
            let end = end.min(end - path_frames + OFF_BY_ONE);
            let tail = Path::ConsStack { head: &backtrace.frames()[*BACKTRACE_PREFIX - OFF_BY_ONE..end], tail: &path };
            let inner = TraceWakerInner::Trace {
                internal: internal,
                trace: *trace,
                path: Path::ConsAnnot { head: &message, tail: &tail },
                path_frames: backtrace.frames().len() - *BACKTRACE_PREFIX,
            };
            fut.as_mut().poll(&mut inner.as_waker().as_context())
        } else {
            fut.as_mut().poll(cx)
        }
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
    fn insert_frames<'a, 'b>(&'a mut self, mut frames: slice::Iter<'b, BacktraceFrame>) -> &'a mut Node {
        if let Some(next) = frames.next_back() {
            self.children.entry(NodeKey::Address(next.ip() as usize)).or_insert(Node::new()).insert_frames(frames)
        } else {
            self
        }
    }
    fn insert<'a, 'b>(&'a mut self, path: &'b Path<'b>) -> &'a mut Node {
        match path {
            Path::ConsAnnot { head, tail } => {
                let head = format!("● {}", head.to_string());
                self.insert(tail).children.entry(NodeKey::Annotation(head)).or_insert(Node::new())
            }
            Path::ConsStack { head, tail } => {
                self.insert(tail).insert_frames(head.iter())
            }
            Path::Nil => {
                self
            }
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
                NodeKey::Address(addr) => resolve_remangle(*addr as *mut c_void),
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

impl<'a> TraceWakerInner<'a> {
    fn trace(&self) -> &Waker {
        let backtrace = Backtrace::new_unresolved();
        match self {
            TraceWakerInner::Trace { internal, trace, path, path_frames } => {
                let mut lock = trace.0.lock();
                let suffix = backtrace.frames().len();
                let suffix = suffix.min(suffix - path_frames + OFF_BY_ONE);
                let path = Path::ConsStack {
                    head: &backtrace.frames()[*WAKE_PREFIX - OFF_BY_ONE..suffix],
                    tail: path,
                };
                lock.node.insert(&path).count += 1;
                internal
            }
            TraceWakerInner::Calibrate { result } => {
                result.set(backtrace).unwrap();
                noop_waker_ref()
            }
        }
    }
    fn clone_impl(&self) -> Waker {
        self.trace().clone()
    }
    fn wake_by_ref_impl(&self) {
        self.trace().wake_by_ref();
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

fn resolve_fn_name(f: usize) -> Option<String> {
    let mut result = None;
    resolve((f + 1) as *mut c_void, |symbol| {
        if result == None {
            result =
                symbol.name()
                    .and_then(|name| name.as_str())
                    .map(|name| name.to_string());
        }
    });
    result
}

fn common_prefix(a: &Backtrace, b: &Backtrace) -> usize {
    a.frames().iter()
        .zip(b.frames().iter())
        .take_while(|(f1, f2)| f1.ip() == f2.ip())
        .count()
}

#[inline(never)]
fn resolve_backtrace_prefix() -> usize {
    #[inline(never)]
    fn resolve_backtrace_prefix_inner() -> Backtrace {
        Backtrace::new_unresolved()
    }
    let outer = Backtrace::new_unresolved();
    let inner = resolve_backtrace_prefix_inner();
    common_prefix(&outer, &inner) + 1
}

fn resolve_wake_prefix() -> usize {
    fn resolve_wake_prefix_inner() -> Backtrace {
        let bt2 = SyncOnceCell::new();
        let inner = TraceWakerInner::Calibrate { result: &bt2 };
        inner.as_waker().as_context().waker().wake_by_ref();
        bt2.into_inner().unwrap()
    }
    let bt1 = SyncOnceCell::new();
    let inner = TraceWakerInner::Calibrate { result: &bt1 };
    inner.as_waker().as_context().waker().wake_by_ref();
    let bt2 = resolve_wake_prefix_inner();
    common_prefix(&bt1.into_inner().unwrap(), &bt2)
}

const OFF_BY_ONE: usize = 0;
lazy_static! {
    static ref BACKTRACE_PREFIX: usize = resolve_backtrace_prefix() ;
    static ref WAKE_PREFIX: usize = resolve_wake_prefix();
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

    #[inline(never)]
    async fn tree3() {
        sleep(Duration::from_millis(1000)).await;
    }

    #[inline(never)]
    async fn tree2() {
        join!(
            trace_annotate(&|f| write!(f, "tree2.a"), tree3()),
            trace_annotate(&|f| write!(f, "tree2.b"), tree3())
        );
    }

    #[inline(never)]
    async fn tree1() {
        join!(
            trace_annotate(&|f| write!(f, "tree1.a"), tree2()),
            trace_annotate(&|f| write!(f, "tree1.b"), tree2())
        );
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
                spawn(tree1());
                sleep(Duration::from_millis(100)).await;
                println!("{}", group.capture().await);
            }
        };
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().on_thread_start(group.on_thread_start()).build().unwrap();
        group.set_current();
        rt.block_on(fut)
    }
}