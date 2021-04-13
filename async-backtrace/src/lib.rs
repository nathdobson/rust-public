#![feature(associated_type_bounds)]
#![feature(option_result_contains)]
#![feature(test)]
#![feature(drain_filter)]
#![allow(unused_imports, unused_variables, dead_code)]
#![feature(iter_intersperse)]
#![feature(never_type)]
#![deny(unused_must_use)]
#![feature(trait_alias)]
#![feature(once_cell)]
#![feature(min_type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![allow(incomplete_features)]
#![feature(future_poll_fn)]

mod remangle;
mod trace;
mod server;
mod lldb_capture;
mod spawn;
mod trace_group;

pub use trace::Trace;
pub use trace_group::TraceGroup;
pub use server::traced_server;
pub use server::traced_main;
pub use spawn::spawn;
pub use spawn::TracedPriorityPool;