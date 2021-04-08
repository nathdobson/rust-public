use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::thread;
use tokio::net::TcpListener;
use backtrace::Backtrace;
use crate::Trace;
use hyper::http::Result;


async fn handler_stacks_async(_req: Request<Body>) -> Result<Response<Body>> {
    let trace = Trace::new().to_string();
    Response::builder()
        .header("content-type", "text/plain; charset=utf-8")
        .body(trace.into())
}

async fn handler_stacks(_req: Request<Body>) -> Result<Response<Body>> {
    todo!()
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

pub fn run_debug_server(port: u16) {
    // pre-load symbol tables.
    thread::Builder::new().name("debug-server-heater".to_string()).spawn(|| { Backtrace::new(); }).unwrap();
    thread::Builder::new().name("debug-server".to_string()).spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            // .worker_threads(2)
            .thread_name("debug-server-worker")
            .enable_io()
            .build().unwrap()
            .block_on(async {
                run_debug_server_async(port).await;
            });
    }).unwrap();
}

pub async fn run_debug_server_async(port: u16) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handler))
    });
    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("debug server error: {}", e);
    }
}