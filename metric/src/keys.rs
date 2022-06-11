use std::fmt::Debug;
use std::hash::Hash;

use tuple_list::Tuple;

use crate::database::{SnapshotCell, SnapshotValue};

pub trait KeyList {}

impl<H, T> KeyList for (H, T) where T: KeyList {}

impl KeyList for () {}

pub trait Keys: Hash + Eq + Clone + Debug + Send + 'static {}

impl<T> Keys for T
where
    T: Hash + Eq + Clone + Debug + Send + 'static,
    T: Tuple,
    T::TupleList: KeyList,
{
}
