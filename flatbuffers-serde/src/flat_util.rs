use core::option::Option;
use core::option::Option::{None, Some};
use std::marker::PhantomData;

use flatbuffers::{Follow, Push, UOffsetT};

use crate::vec_slice::VecSlice;

pub struct FlatUnit;

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct Flat128(pub u128);

pub type VariantT = u16;

pub struct FlatUnion<'a> {
    pub buf: &'a [u8],
    pub loc: usize,
}

impl Push for Flat128 {
    type Output = u128;
    fn push(&self, dst: &mut [u8], _rest: &[u8]) { dst.copy_from_slice(&self.0.to_le_bytes()) }
}

impl<'de> Follow<'de> for Flat128 {
    type Inner = u128;

    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        u128::from_le_bytes(buf[loc..loc + 16].try_into().unwrap())
    }
}

impl Push for FlatUnit {
    type Output = ();
    fn push(&self, dst: &mut [u8], _rest: &[u8]) {}
}

pub struct FollowOrNull<T>(T);

impl<'de, T: Follow<'de>> Follow<'de> for FollowOrNull<T> {
    type Inner = Option<T::Inner>;

    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        if UOffsetT::follow(buf, loc) == 0 {
            None
        } else {
            Some(T::follow(buf, loc))
        }
    }
}

impl<'de> Follow<'de> for FlatUnit {
    type Inner = ();

    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner { () }
}

impl<'a> Follow<'a> for FlatUnion<'a> {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner { FlatUnion { buf, loc } }
}
