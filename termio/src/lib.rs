#![feature(half_open_range_patterns)]
#![feature(arc_new_cyclic)]
#![feature(once_cell)]
#![feature(raw)]
#![feature(exclusive_range_pattern)]
#![feature(str_internals)]
#![feature(fmt_internals)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![feature(arbitrary_self_types)]
#![feature(toowned_clone_into)]
#![feature(specialization)]
#![feature(box_syntax)]
#![feature(never_type)]
#![feature(raw_ref_op)]
#![feature(async_stream)]
#![feature(future_poll_fn)]
#![deny(unused_must_use)]
#![allow(
    unused_imports,
    unused_variables,
    dead_code,
    unreachable_code,
    incomplete_features
)]

use std::iter::FromIterator;
use std::ops::DerefMut;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate util;
#[macro_use]
extern crate strum_macros;

pub mod advance;
pub mod canvas;
pub mod color;
pub mod gui;
pub mod image;
pub mod input;
pub mod line;
pub mod output;
pub mod screen;
pub mod string;
pub mod symbols;
pub mod tokenizer;
pub mod writer;

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}
