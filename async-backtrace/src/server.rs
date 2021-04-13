use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn, Service};
use std::{thread};
use tokio::net::TcpListener;
use backtrace::Backtrace;
use crate::{Trace, lldb_capture};
use hyper::http::Result;
use std::time::Instant;
use std::fmt::Write;
use std::str::FromStr;
use std::task::{Context, Poll};
use std::future::Future;
use std::pin::Pin;
use async_util::futureext::FutureExt;
use crate::trace_group::TraceGroup;

struct MakeDebugService(TraceGroup);

struct DebugService(TraceGroup);

impl<T> Service<T> for MakeDebugService {
    type Response = DebugService;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Send + Future<Output=Result<DebugService>>>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<()>> { Poll::Ready(Ok(())) }

    fn call(&mut self, _: T) -> Self::Future {
        let group = self.0.clone();
        async move { Ok(DebugService(group)) }.boxed()
    }
}


impl Service<Request<Body>> for DebugService {
    type Response = Response<Body>;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Send + Future<Output=Result<Self::Response>>>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<()>> { return Poll::Ready(Ok(())); }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let group = self.0.clone();
        async move {
            match (req.method(), req.uri().path()) {
                (&Method::GET, "/stacks_async") => handler_stacks_async(&group, req).await,
                (&Method::GET, "/stacks") => handler_stacks(req).await,
                _ => not_found()
            }
        }.boxed()
    }
}

async fn handler_stacks_async(group: &TraceGroup, _req: Request<Body>) -> Result<Response<Body>> {
    let start = Instant::now();
    let mut trace = group.capture().to_string();
    writeln!(trace, "Async capture in {:?}", start.elapsed()).unwrap();
    Response::builder()
        .header("content-type", "text/plain; charset=utf-8")
        .body(trace.into())
}

async fn handler_stacks(_req: Request<Body>) -> Result<Response<Body>> {
    Response::builder()
        .header("content-type", "text/plain; charset=utf-8")
        .body(lldb_capture::capture().await.unwrap().into())
}

fn not_found() -> Result<Response<Body>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body((b"Not found" as &[u8]).into())
}

pub fn traced_server(group: TraceGroup, addr: String) {
    // pre-load symbol tables.
    thread::Builder::new().name("debug-server-heater".to_string()).spawn(|| { Backtrace::new(); }).unwrap();
    thread::Builder::new().name("debug-server".to_string()).spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            // .worker_threads(2)
            .thread_name("debug-server-worker")
            .enable_io()
            .build().unwrap()
            .block_on(async {
                traced_server_async(group, addr).await;
            });
    }).unwrap();
}

pub async fn traced_server_async(group: TraceGroup, addr: String) {
    let addr = SocketAddr::from_str(&addr).unwrap();
    let server = Server::bind(&addr).serve(MakeDebugService(group));
    if let Err(e) = server.await {
        eprintln!("debug server error: {}", e);
    }
}

pub fn traced_main<F: 'static + Send + Future<Output: 'static + Send>>(addr: String, fut: F) -> F::Output {
    let group = TraceGroup::new();
    traced_server(group.clone(), addr);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .on_thread_start(group.on_thread_start())
        .build()
        .unwrap();
    let (remote, handle) = fut.into_remote();
    rt.spawn(group.push(remote));
    rt.block_on(handle)
}