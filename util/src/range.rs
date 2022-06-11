use std::cmp;
use std::ops::Range;

use num::CheckedAdd;

pub trait RangeExt {
    fn intersects(&self, other: &Self) -> bool;
    fn sub_range(&self, other: &Self) -> Self;
    fn sub_range_truncated(&self, other: &Self) -> Self;
}

impl<T> RangeExt for Range<T>
where
    T: Copy + Ord + CheckedAdd,
{
    fn intersects(&self, other: &Self) -> bool {
        cmp::max(self.start, other.start) < cmp::min(self.end, other.end)
    }
    fn sub_range(&self, other: &Self) -> Self {
        assert!(other.start <= other.end);
        let start = other.start.checked_add(&self.start).unwrap();
        let end = other.end.checked_add(&self.start).unwrap();
        assert!(end <= self.end);
        assert!(self.start <= start);
        start..end
    }
    fn sub_range_truncated(&self, other: &Self) -> Self {
        assert!(other.start <= other.end);
        let mut start = other.start.checked_add(&self.start).unwrap();
        let mut end = other.end.checked_add(&self.start).unwrap();
        start = cmp::max(start, self.start);
        start = cmp::min(start, self.end);
        end = cmp::max(end, self.start);
        end = cmp::min(end, self.end);
        start..end
    }
}

#[test]
fn test_range_ext() {
    assert_eq!((10..20).sub_range(&(2..3)), 12..13);
}

#[test]
#[should_panic]
fn test_range_ext_swap() { (10..20).sub_range(&(3..2)); }

#[test]
#[should_panic]
fn test_range_ext_out_of_range() { (10..20).sub_range(&(5..15)); }

#[test]
#[should_panic]
fn test_range_ext_overflow() { (10..20).sub_range(&(isize::max_value()..isize::max_value())); }
