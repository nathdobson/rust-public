use std::collections::HashMap;
use crate::screen::LineSetting;
use util::grid::Grid;
use util::itertools::Itertools2;
use crate::gui::div::{Div, DivRc};

#[derive(Debug)]
pub struct Constraint {
    pub max_size: Option<(isize, isize)>,
}

pub struct Layout {
    pub size: (isize, isize),
    pub line_settings: HashMap::<isize, LineSetting>,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum Rounding {
    Int,
    Even,
    Odd,
}

#[derive(Copy, Clone, Debug)]
pub struct Align {
    pub(crate) percent: f64,
    pub(crate) rounding: Rounding,
}

pub const LEFT: Align = Align::new(0.0);
pub const TOP: Align = Align::new(0.0);
pub const CENTER: Align = Align::new(0.5);
pub const RIGHT: Align = Align::new(1.0);
pub const BOTTOM: Align = Align::new(1.0);

impl Align {
    pub const fn new(x: f64) -> Self {
        Align {
            percent: x,
            rounding: Rounding::Int,
        }
    }
    pub fn odd(mut self) -> Self {
        self.rounding = Rounding::Odd;
        self
    }
    pub fn even(mut self) -> Self {
        self.rounding = Rounding::Even;
        self
    }
    pub fn align(&self, start: isize, inner_size: isize, outer_size: isize) -> isize {
        let x = (start as f64 + self.percent * (outer_size as f64 - inner_size as f64)).round() as isize;
        match self.rounding {
            Rounding::Int => x,
            Rounding::Even => x & !1,
            Rounding::Odd => (x & !1) + 1,
        }
    }
}

impl Constraint {
    pub fn from_max(max_size: (isize, isize)) -> Self {
        assert!(max_size.0 >= 0);
        assert!(max_size.1 >= 0);
        Constraint { max_size: Some(max_size) }
    }
    pub fn none() -> Self {
        Constraint { max_size: None }
    }

}
