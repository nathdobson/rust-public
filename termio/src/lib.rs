#![feature(half_open_range_patterns, exclusive_range_pattern, str_internals, fmt_internals, coerce_unsized, unsize, arbitrary_self_types, toowned_clone_into)]
#![allow(unused_imports, unused_variables, dead_code, unreachable_code)]

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

pub mod output;
pub mod input;
pub mod tokenizer;
pub mod color;
pub mod canvas;
pub mod gui;
pub mod screen;
pub mod writer;
pub mod string;

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}
