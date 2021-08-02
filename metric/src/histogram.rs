use crate::values::Values;
use ordered_float::OrderedFloat;
use std::ops::Range;
use crate::database::Snapshot;
use std::iter;
use crate::database::SnapshotRow;

#[derive(Debug)]
pub struct Buckets {
    values: Vec<OrderedFloat<f64>>,
}

#[derive(Default, Clone, Debug)]
pub struct Histogram(Vec<f64>);

#[derive(Debug)]
pub struct Point {
    pub value: f64,
    pub weight: f64,
}

impl Buckets {
    pub fn exponential(start: f64, ratio: f64, count: usize) -> Self {
        assert!(start > 0.0);
        assert!(ratio > 1.0);
        assert!(count > 0);
        let mut result = vec![];
        let mut value = start;
        for _ in 0..count - 1 {
            result.push(OrderedFloat(value));
            value *= ratio;
        }
        Buckets { values: result }
    }
}

impl Values for Buckets {
    type Set = Histogram;
    type Point = Point;

    fn add_point(&self, set: &mut Self::Set, point: &Self::Point) {
        let index = self.values.binary_search(&OrderedFloat(point.value)).into_ok_or_err();
        set.0.resize((index + 1).max(set.0.len()), 0.0);
        set.0[index] += point.weight;
    }

    fn add_set(&self, set: &mut Self::Set, other: &Self::Set) {
        set.0.resize(self.values.len().max(other.0.len()), 0.0);
        for (x, y) in set.0.iter_mut().zip(other.0.iter()) {
            *x += *y;
        }
    }

    fn clear(&self, set: &mut Self::Set) {
        set.0.clear();
    }

}
