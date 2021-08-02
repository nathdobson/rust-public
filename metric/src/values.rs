use std::sync::{Arc};
use parking_lot::Mutex;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;
use rusqlite::Connection;
use crate::database::Snapshot;

pub trait Values: Debug + Send + 'static + Sync {
    type Set: Default + Clone + Debug + Send + 'static;
    type Point: Debug;
    fn add_point(&self, set: &mut Self::Set, point: &Self::Point);
    fn add_set(&self, set: &mut Self::Set, other: &Self::Set);
    fn clear(&self, set: &mut Self::Set);
}

impl<T> Values for T where T: Deref + Debug + Sync + Send + 'static, T::Target: Values {
    type Set = <<T as Deref>::Target as Values>::Set;
    type Point = <<T as Deref>::Target as Values>::Point;
    fn add_point(&self, set: &mut Self::Set, point: &Self::Point) { self.deref().add_point(set, point); }
    fn add_set(&self, set: &mut Self::Set, other: &Self::Set) { self.deref().add_set(set, other); }
    fn clear(&self, set: &mut Self::Set) { self.deref().clear(set); }
}

