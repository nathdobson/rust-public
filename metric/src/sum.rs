use std::fmt::Debug;
use std::iter;
use std::marker::PhantomData;
use std::ops::AddAssign;

use rusqlite::Connection;

use crate::database::{Snapshot, SnapshotCell, SnapshotValue};
use crate::values::Values;

#[derive(Default, Clone, Debug)]
struct SumStats<T>(PhantomData<T>);

impl<T> Values for SumStats<T>
where
    T: Default,
    for<'a> T: AddAssign<&'a T> + Clone + Debug + Send + Sync + 'static,
    SnapshotValue: From<T>,
{
    type Set = T;
    type Point = T;
    fn add_point(&self, set: &mut Self::Set, point: &Self::Point) { *set += point; }
    fn add_set(&self, set: &mut Self::Set, other: &Self::Set) { *set += other; }
    fn clear(&self, set: &mut Self::Set) { *set = T::default(); }
}
