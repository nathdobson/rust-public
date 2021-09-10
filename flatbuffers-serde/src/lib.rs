#![feature(never_type)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![deny(unused_must_use)]
#![feature(specialization)]
#![allow(incomplete_features)]
#![allow(unreachable_code)]

mod ser;
mod de;

use flatbuffers::{FlatBufferBuilder, WIPOffset, UnionWIPOffset, InvalidFlatbuffer, Push, Follow, UOffsetT};
use serde::{Serializer, Serialize, Deserializer};
use std::fmt::{Display, Debug, Formatter};
use std::error::Error;
use serde::ser::{SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeMap, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant};
use std::io::Cursor;
use serde::de::{Visitor, SeqAccess, DeserializeSeed};
use std::marker::PhantomData;

pub mod test_generated {
    include!(concat!(env!("OUT_DIR"), "/test_generated.rs"));
}

pub mod any_raw_generated {
    use flatbuffers::{Follow, Verifiable, Verifier, InvalidFlatbuffer, FlatBufferBuilder, WIPOffset};
    use std::fmt::{Debug, Formatter};
    use serde::{Serialize, Deserialize};
    use crate::ser::{SerializeError, Serializer};
    use crate::de::{DeserializeError, Deserializer, IdentityDeserializer};
    use crate::ser::Stack;

    pub struct AnyFlatRaw<'a> {
        buf: &'a [u8],
        loc: usize,
    }

    impl<'a> Follow<'a> for AnyFlatRaw<'a> {
        type Inner = AnyFlatRaw<'a>;
        fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
            AnyFlatRaw { buf, loc }
        }
    }

    impl<'a> Verifiable for AnyFlatRaw<'a> {
        fn run_verifier(v: &mut Verifier, pos: usize) -> Result<(), InvalidFlatbuffer> {
            Ok(())
        }
    }

    impl<'a> Debug for AnyFlatRaw<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("AnyFlatRaw").finish_non_exhaustive()
        }
    }

    impl<'a> AnyFlatRaw<'a> {
        pub fn serialize<'b, T: Serialize>(
            fbb: &'b mut FlatBufferBuilder<'a>,
            value: &T,
        ) -> Result<WIPOffset<AnyFlatRaw<'a>>, SerializeError> {
            let mut stack = Stack::new();
            let mut serializer = Serializer::new(fbb, &mut stack);
            let value = serializer.serialize_to_offset(value)?;
            Ok(WIPOffset::new(value.value()))
        }
        pub fn deserialize<T: Deserialize<'a>>(&self) -> Result<T, DeserializeError> {
            T::deserialize(Deserializer::<IdentityDeserializer>::follow(self.buf, self.loc))
        }
    }
}

pub mod any_generated {
    include!(concat!(env!("OUT_DIR"), "/any_generated.rs"));
}

struct EmptyPush;

struct U128(u128);

impl Push for U128 {
    type Output = u128;
    fn push(&self, dst: &mut [u8], _rest: &[u8]) {
        dst.copy_from_slice(&self.0.to_le_bytes())
    }
}

impl<'de> Follow<'de> for U128 {
    type Inner = u128;

    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        u128::from_le_bytes(buf[loc..loc + 16].try_into().unwrap())
    }
}

impl Push for EmptyPush {
    type Output = ();
    fn push(&self, dst: &mut [u8], _rest: &[u8]) {}
}

struct FollowOrNull<T>(T);

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

type VariantT = u16;