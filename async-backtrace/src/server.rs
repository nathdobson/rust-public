use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::thread;
use tokio::net::TcpListener;
use backtrace::Backtrace;
use crate::{Trace, lldb_capture};
use hyper::http::Result;
use std::time::Instant;
use std::fmt::Write;
use std::str::FromStr;

async fn handler_stacks_async(_req: Request<Body>) -> Result<Response<Body>> {
    let start = Instant::now();
    let mut trace = Trace::new().to_string();
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

async fn handler(req: Request<Body>) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/stacks_async") => handler_stacks_async(req).await,
        (&Method::GET, "/stacks") => handler_stacks(req).await,
        _ => not_found()
    }
}

fn not_found() -> Result<Response<Body>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body((b"Not found" as &[u8]).into())
}

pub fn run_debug_server(addr: String) {
    // pre-load symbol tables.
    thread::Builder::new().name("debug-server-heater".to_string()).spawn(|| { Backtrace::new(); }).unwrap();
    thread::Builder::new().name("debug-server".to_string()).spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            // .worker_threads(2)
            .thread_name("debug-server-worker")
            .enable_io()
            .build().unwrap()
            .block_on(async {
                run_debug_server_async(addr).await;
            });
    }).unwrap();
}

pub async fn run_debug_server_async(addr: String) {
    let addr = SocketAddr::from_str(&addr).unwrap();
    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handler))
    });
    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("debug server error: {}", e);
    }
}
