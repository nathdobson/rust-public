#![feature(associated_type_bounds)]
#![feature(option_result_contains)]
#![feature(test)]
#![feature(drain_filter)]
#![allow(unused_imports, unused_variables, dead_code)]
#![feature(iter_intersperse)]
#![feature(never_type)]
#![deny(unused_must_use)]

mod remangle;
mod capture;
mod server;
mod lldb_capture;

pub use capture::Trace;
pub use capture::spawn;
pub use server::run_debug_server;
