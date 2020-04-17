#![feature(half_open_range_patterns, exclusive_range_pattern, str_internals, fmt_internals)]
#![allow(unused_imports, unused_variables, dead_code, unreachable_code)]

use std::iter::FromIterator;
use std::ops::DerefMut;

pub mod input;
pub mod prompt;
pub mod output;
pub mod tokenizer;
pub mod color;
pub mod canvas;
pub mod gui;

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq, Hash, Copy, Clone)]
pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}
