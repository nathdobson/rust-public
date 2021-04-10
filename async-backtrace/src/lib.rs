#![feature(associated_type_bounds)]
#![feature(option_result_contains)]
#![feature(test)]
#![feature(drain_filter)]
#![allow(unused_imports, unused_variables, dead_code)]
#![feature(iter_intersperse)]
#![feature(never_type)]
#![deny(unused_must_use)]
#![feature(trait_alias)]

mod remangle;
mod async_capture;
mod server;
mod lldb_capture;

pub use async_capture::Trace;
pub use async_capture::spawn;
pub use server::run_debug_server;
