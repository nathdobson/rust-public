#![feature(half_open_range_patterns, exclusive_range_pattern, str_internals, fmt_internals, coerce_unsized, unsize)]
#![allow(unused_imports, unused_variables, dead_code, unreachable_code)]

use std::iter::FromIterator;
use std::ops::DerefMut;

#[macro_use]
pub mod write;

#[macro_use]
extern crate serde_derive;

extern crate serde;

pub mod output;
pub mod input;
pub mod prompt;
pub mod tokenizer;
pub mod color;
pub mod canvas;
pub mod gui;

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}
