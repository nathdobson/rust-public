use std::fmt::{Display, Formatter, Debug};
use std::fmt;

pub trait SyncDisplay: Sync + Fn(&mut Formatter) -> fmt::Result {}

impl<T> SyncDisplay for T where T: Sync + Fn(&mut Formatter) -> fmt::Result {}

impl<'a> Display for &'a dyn SyncDisplay {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        self(fmt)
    }
}