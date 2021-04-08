#![feature(associated_type_bounds)]
#![feature(option_result_contains)]
#![feature(test)]
#![feature(drain_filter)]
#![allow(unused_must_use, unused_imports, unused_variables)]
#![feature(iter_intersperse)]
#![feature(never_type)]

mod remangle;
mod capture;

pub use capture::Trace;
pub use capture::spawn;

use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use std::thread;
use tokio::net::TcpListener;

async fn hello_world(_req: Request<Body>) -> Result<Response<Body>, hyper::http::Error> {
    Response::builder()
        .header("content-type", "text/plain; charset=utf-8")
        .body(Trace::new().to_string().into())
}

pub fn run_debug_server() {
    thread::Builder::new().name("debug-server".to_string()).spawn(|| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("debug-server-worker")
            .enable_io()
            .build().unwrap()
            .block_on(async {
                run_debug_server_async().await;
            });
    });
}

pub async fn run_debug_server_async() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 9999));
    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(hello_world))
    });
    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("debug server error: {}", e);
    }
}