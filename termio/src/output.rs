#![allow(non_snake_case, non_upper_case_globals)]

use std::borrow::Borrow;
use std::fmt;
use std::fmt::{Arguments, Display, Error, Formatter};
use std::fmt::rt::v1::Argument;
use std::io;
use std::io::{BufWriter, Write};
use std::ops::Deref;

use crate::Direction;
use crate::color::Color;
use std::collections::VecDeque;

pub struct AsDisplay<F: Fn(&mut Formatter) -> Result<(), Error>>(pub F);

impl<F: Fn(&mut Formatter) -> Result<(), Error>> Display for AsDisplay<F> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        self.0(f)
    }
}

macro_rules! concat {
    ($($es:expr),*) => {
        AsDisplay(move |fmt|{
            $(
                write!(fmt,"{}",$es)?;
            )*
            Ok(())
        })
    };
}

pub fn OneParameter(prefix: &'static str, param: usize, suffix: &'static str) -> impl Display {
    AsDisplay(move |f| {
        match param {
            0 => Ok(()),
            1 => write!(f, "{}{}", prefix, suffix),
            _ => write!(f, "{}{}{}", prefix, param, suffix),
        }
    })
}

pub fn MoveDirection(dir: Direction, len: usize) -> impl Display {
    let suffix = match dir {
        Direction::Up => "A",
        Direction::Down => "B",
        Direction::Right => "C",
        Direction::Left => "D",
    };
    OneParameter("\x1B[", len, suffix)
}

pub fn MoveVector(x: isize, y: isize) -> impl Display {
    AsDisplay(move |f| {
        if x < 0 {
            write!(f, "{}", MoveDirection(Direction::Left, -x as usize))?
        } else if x > 0 {
            write!(f, "{}", MoveDirection(Direction::Right, x as usize))?
        }
        if y < 0 {
            write!(f, "{}", MoveDirection(Direction::Up, -y as usize))?
        } else if y > 0 {
            write!(f, "{}", MoveDirection(Direction::Down, y as usize))?
        }
        Ok(())
    })
}

pub fn CursorPosition(x: isize, y: isize) -> impl Display { concat!("\x1b[", y, ";", x, "H") }

pub fn Delete(count: usize) -> impl Display { OneParameter("\x1B[", count, "P") }

pub fn Insert(count: usize) -> impl Display { OneParameter("\x1B[", count, "@") }

pub fn Column(x: usize) -> impl Display { OneParameter("\x1B[", x, "G") }

pub fn MoveWindow(x: usize, y: usize) -> impl Display { concat!("\x1B[3;", x, ";", y, "t") }

pub fn ResizeWindow(w: usize, h: usize) -> impl Display { concat!( "\x1B[4;", h, ";", w, "t") }

pub fn Foreground(color: Color) -> impl Display {
    AsDisplay(move |f| {
        match color.into_u8() {
            None => write!(f, "\x1b[39m"),
            Some(x) => write!(f, "\x1B[38;5;{}m", x),
        }
    })
}

pub fn Background(color: Color) -> impl Display {
    AsDisplay(move |f| {
        match color.into_u8() {
            None => write!(f, "\x1b[49m"),
            Some(x) => write!(f, "\x1B[48;5;{}m", x),
        }
    })
}

pub const VideoPush: &'static str = "\x1b[#{";
pub const VideoPop: &'static str = "\x1b[#}";
pub const VideoNormal: &'static str = "\x1b[0m";

pub const DoubleHeightTop: &'static str = "\x1B#3";
pub const DoubleHeightBottom: &'static str = "\x1B#4";
pub const SingleWidthLine: &'static str = "\x1B#5";

pub const DeleteLine: &'static str = "\x1b[2K";
pub const NoFormat: &'static str = "\x1b[0m";

pub const CursorHide: &'static str = "\x1B[?25l";
pub const CursorShow: &'static str = "\x1B[?25h";
pub const CursorSave: &'static str = "\x1B7";
pub const CursorRestore: &'static str = "\x1B8";
pub const CursorStyle0: &'static str = "\x1B[0 q";

pub const AllMotionTrackingEnable: &'static str = "\x1B[?1003h";
pub const AllMotionTrackingDisable: &'static str = "\x1B[?1003l";

pub const FocusTrackingEnable: &'static str = "\x1B[?1004h";
pub const FocusTrackingDisable: &'static str = "\x1B[?1004l";

pub const AlternateEnable: &'static str = "\x1B[?1049h";
pub const AlternateDisable: &'static str = "\x1B[?1049l";

pub const ReportWindowPosition: &'static str = "\x1B[13t";
pub const ReportWindowSize: &'static str = "\x1B[14t";
pub const ReportTextAreaSize: &'static str = "\x1B[18t";
pub const ScreenSize: &'static str = "\x1B[19t";
pub const RaiseWindow: &'static str = "\x1B[5t";
pub const LowerWindow: &'static str = "\x1B[6t";
pub const ReportVisibleState: &'static str = "\x1B[11t";
pub const MinimizeWindow: &'static str = "\x1B[2t";
pub const MaximizeWindow: &'static str = "\x1B[1t";
pub const EraseAll: &'static str = "\x1B[2J";

pub fn ScrollRegion(start: usize, end: usize) -> impl Display { concat!("\x1B[", start, ";", end, "r") }


pub fn draw_box(c11: bool, c21: bool, c12: bool, c22: bool) -> char {
    match (c11, c21, c12, c22) {
        (false, false, false, false) => ' ',

        (true, false, false, false) => '▘',
        (false, true, false, false) => '▝',
        (false, false, true, false) => '▖',
        (false, false, false, true) => '▗',

        (true, true, false, false) => '▀',
        (false, false, true, true) => '▄',
        (true, false, true, false) => '▌',
        (false, true, false, true) => '▐',

        (true, false, false, true) => '▚',
        (false, true, true, false) => '▞',

        (false, true, true, true) => '▟',
        (true, false, true, true) => '▙',
        (true, true, false, true) => '▜',
        (true, true, true, false) => '▛',

        (true, true, true, true) => '█',
    }
}