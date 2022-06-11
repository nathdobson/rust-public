use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use flatbuffers::{
    root, root_unchecked, FlatBufferBuilder, Follow, ForwardsUOffset, InvalidFlatbuffer,
    Verifiable, Verifier, VerifierOptions, WIPOffset,
};

use crate::flat_util::FlatUnion;
use crate::vec_slice::VecSlice;

pub struct FlatBuffer<T> {
    data: VecSlice,
    phantom: PhantomData<T>,
}

impl<T> FlatBuffer<T> {
    pub fn new<'a>(mut builder: FlatBufferBuilder<'a>, finish: WIPOffset<T>) -> FlatBuffer<T> {
        builder.finish(finish, None);
        let (vec, head) = builder.collapse();
        FlatBuffer {
            data: VecSlice::from_vec(vec, head),
            phantom: PhantomData,
        }
    }
    pub fn build(f: impl FnOnce(&mut FlatBufferBuilder<'static>) -> WIPOffset<T>) -> Self {
        let mut fbb = FlatBufferBuilder::new();
        let root = f(&mut fbb);
        Self::new(fbb, root)
    }
    pub fn from_vec_slice(data: VecSlice) -> Result<Self, InvalidFlatbuffer>
    where
        T: Verifiable,
    {
        let opts = VerifierOptions::default();
        let mut v = Verifier::new(&opts, data.as_ref());
        <ForwardsUOffset<T>>::run_verifier(&mut v, 0)?;
        Ok(FlatBuffer {
            data,
            phantom: PhantomData,
        })
    }
    pub fn into_inner(self) -> VecSlice { self.data }
    pub fn as_slice(&self) -> &[u8] { self.data.as_ref() }
    pub fn root<'a>(&'a self) -> T::Inner
    where
        T: Follow<'a>,
    {
        unsafe { root_unchecked::<T>(self.data.as_ref()) }
    }
    pub fn root_covariant<'a>(&'a self) -> <T::Super<'a> as Follow<'a>>::Inner
    where
        T: Covariant,
        T::Super<'a>: Follow<'a>,
    {
        unsafe { root_unchecked::<T::Super<'a>>(self.data.as_ref()) }
    }
}

pub unsafe trait Covariant {
    type Super<'b>
    where
        Self: 'b;
}

impl<T> Debug for FlatBuffer<T>
where
    T: Covariant,
    for<'a> T::Super<'a>: Follow<'a>,
    for<'a> <T::Super<'a> as Follow<'a>>::Inner: Debug,
{
    fn fmt<'a, 'b>(&'a self, f: &'a mut Formatter<'b>) -> std::fmt::Result {
        write!(f, "{:?}", self.root_covariant())
    }
}
