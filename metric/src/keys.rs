use crate::database::{SnapshotCell, SnapshotValue};
use tuple_list::Tuple;
use std::hash::Hash;
use std::fmt::Debug;

pub trait KeyList {}

impl<H, T> KeyList for (H, T) where T: KeyList {}

impl KeyList for () {}

pub trait Keys: Hash + Eq + Clone + Debug + Send + 'static {}

impl<T> Keys for T where T: Hash + Eq + Clone + Debug + Send + 'static, T: Tuple, T::TupleList: KeyList {}
