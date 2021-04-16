#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(unused_must_use)]

#![feature(never_type)]
#![feature(negative_impls)]
#![feature(unboxed_closures)]
#![feature(once_cell)]
#![feature(arbitrary_self_types)]
#![allow(unused_imports)]
#![feature(slice_ptr_len)]
#![feature(box_syntax)]
#![feature(arc_new_cyclic)]
#![feature(map_first_last)]
#![feature(async_stream)]
#![feature(slice_concat_trait)]
#![feature(generic_associated_types)]
#![feature(future_poll_fn)]
#![feature(generator_trait)]
#![feature(generators)]
#![feature(trait_alias)]
#![allow(incomplete_features)]
#![feature(backtrace)]

//pub mod bytes;
pub mod coop;
pub mod promise;
pub mod parser;
pub mod waker;
pub mod pipe;
pub mod timer;
pub mod priority;
pub mod dirty;
pub mod mut_future;
pub mod spawn;
pub mod join;
pub mod futureext;
pub mod fused;
