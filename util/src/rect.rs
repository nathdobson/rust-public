use std::ops::Range;

use itertools::Itertools;

use crate::range::RangeExt;

#[derive(Eq, PartialEq, PartialOrd, Hash, Copy, Clone, Debug, Default)]
pub struct Rect {
    xs: (isize, isize),
    ys: (isize, isize),
}

impl Rect {
    pub fn from_ranges(xs: Range<isize>, ys: Range<isize>) -> Rect {
        assert!(xs.start <= xs.end);
        assert!(ys.start <= ys.end);
        Rect {
            xs: (xs.start, xs.end),
            ys: (ys.start, ys.end),
        }
    }
    pub fn xs(&self) -> Range<isize> { self.xs.0..self.xs.1 }
    pub fn ys(&self) -> Range<isize> { self.ys.0..self.ys.1 }
    pub fn intersects(&self, other: &Rect) -> bool {
        self.xs().intersects(&other.xs()) && self.ys().intersects(&other.ys())
    }
    pub fn position(&self) -> (isize, isize) { (self.xs.0, self.ys.0) }
    pub fn size(&self) -> (isize, isize) { (self.xs.1 - self.xs.0, self.ys.1 - self.ys.0) }
    pub fn translate(&self, point: (isize, isize)) -> Option<(isize, isize)> {
        let (x, y) = (
            self.xs.0.checked_add(point.0).unwrap(),
            self.ys.0.checked_add(point.1).unwrap(),
        );
        if !self.xs().contains(&x) || !self.ys().contains(&y) {
            None
        } else {
            Some((x, y))
        }
    }
    pub fn sub_rectangle(&self, other: &Rect) -> Rect {
        Rect::from_ranges(
            self.xs().sub_range(&other.xs()),
            self.ys().sub_range(&other.ys()),
        )
    }
    pub fn sub_rectangle_truncated(&self, other: &Rect) -> Rect {
        Rect::from_ranges(
            self.xs().sub_range_truncated(&other.xs()),
            self.ys().sub_range_truncated(&other.ys()),
        )
    }
    pub fn contains(&self, (x, y): (isize, isize)) -> bool {
        self.xs().contains(&x) && self.ys().contains(&y)
    }
    pub fn from_position_size(position: (isize, isize), size: (isize, isize)) -> Self {
        assert!(size.0 >= 0);
        assert!(size.1 >= 0);
        Rect::from_ranges(
            position.0..position.0 + size.0,
            position.1..position.1 + size.1,
        )
    }
    pub fn points_by_row(&self) -> impl Iterator<Item = (isize, isize)> {
        self.ys().cartesian_product(self.xs()).map(|(y, x)| (x, y))
    }
}
