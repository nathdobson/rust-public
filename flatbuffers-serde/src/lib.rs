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

use flatbuffers::{FlatBufferBuilder, WIPOffset, UnionWIPOffset, InvalidFlatbuffer, Push, Follow};
use serde::{Serializer, Serialize, Deserializer};
use std::fmt::{Display, Debug, Formatter};
use std::error::Error;
use serde::ser::{SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeMap, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant};
use std::io::Cursor;
use serde::de::{Visitor, SeqAccess, DeserializeSeed};

pub mod test_generated {
    include!(concat!(env!("OUT_DIR"), "/test_generated.rs"));
}

pub mod any_raw_generated {
    use flatbuffers::{Follow, Verifiable, Verifier, InvalidFlatbuffer, FlatBufferBuilder, WIPOffset};
    use std::fmt::{Debug, Formatter};
    use serde::{Serialize, Deserialize};
    use crate::ser::{SerializeError, Serializer};
    use crate::de::{DeserializeError, Deserializer};
    use crate::ser::Stack;
    use crate::de::IsIdentity;


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
            T::deserialize(Deserializer::<IsIdentity>::follow(self.buf, self.loc))
        }
    }
}

pub mod any_generated {
    include!(concat!(env!("OUT_DIR"), "/any_generated.rs"));
}


