use util::web::start_web_server;

fn main() { start_web_server("0.0.0.0:8000", "./examples/web.rs".as_ref()).unwrap(); }
