use std::fmt::Debug;

use rand_core::{RngCore, SeedableRng};

pub trait RngDyn: RngCore + Sync + Send + Debug + 'static {
    fn fork(&mut self) -> BoxRng;
}

impl<T> RngDyn for T
where
    T: RngCore + Sync + Send + SeedableRng + Debug + 'static,
{
    fn fork(&mut self) -> BoxRng { Box::new(Self::from_rng(self).unwrap()) }
}

pub type BoxRng = Box<dyn RngDyn>;
