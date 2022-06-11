#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![feature(arbitrary_self_types)]
#![feature(map_first_last)]
#![feature(option_result_contains)]

pub mod puzzle;
pub mod view;

use std::env::args;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::{fmt, fs, io};

use itertools::Itertools;
use termio::color::{BaseColor, Color};
use termio::gui::gui::Gui;
use termio::gui::run_local;
use termio::screen::Style;
use util::mutrc::MutRc;

use crate::puzzle::Puzzle;
use crate::view::PuzzleDiv;

fn main() {
    run_local(|tree| {
        let puzzle = Puzzle::parse(&args().nth(1).unwrap());
        let mut gui = Gui::new(tree.clone(), PuzzleDiv::new(tree, puzzle));
        gui.set_background(Style {
            background: Color::Bright(BaseColor::White),
            ..Style::default()
        });
        MutRc::new(gui)
    });
}
