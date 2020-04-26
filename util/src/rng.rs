use rand_core::{RngCore, SeedableRng};

pub trait RngDyn: RngCore + Sync + Send + 'static {
    fn fork(&mut self) -> BoxRng;
}

impl<T> RngDyn for T where T: RngCore + Sync + Send + SeedableRng + 'static {
    fn fork(&mut self) -> BoxRng {
        Box::new(Self::from_rng(self).unwrap())
    }
}

pub type BoxRng = Box<dyn RngDyn>;
