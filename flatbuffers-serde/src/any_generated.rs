use serde::{Serialize, Deserialize};
use flatbuffers::{FlatBufferBuilder, WIPOffset, Verifiable, Verifier, InvalidFlatbuffer, Follow, Push, Table, VOffsetT, ForwardsUOffset, TableUnfinishedWIPOffset, UnionWIPOffset};
use crate::tag::{HasTypeTag, TypeTag, HasFlatTypeTag};
use crate::de::error::Error;
use std::fmt::{Debug, Formatter, Display};
use crate::de::identity::IdentityDeserializer;
use crate::de::wrapper::Deserializer;
use crate::ser::wrapper::{Serializer, Stack};
use sha2::{Sha256, Digest};
use std::convert::TryInto;
use core::mem;
use crate::ser::serialize_raw;
use crate::flat_util::FollowLazy;
use crate::de::deserialize_raw;
use std::default::default;
use std::any::type_name;
use registry::registry;

pub struct AnyFlat<'a> {
    table: flatbuffers::Table<'a>,
}

#[derive(Debug)]
pub struct TypeMismatch {
    from: Result<&'static TypeTag, TypeTagHash>,
    to: &'static TypeTag,
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Hash)]
pub struct TypeTagHash([u8; 16]);

impl std::error::Error for TypeMismatch {}

impl Display for TypeMismatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.from {
            Ok(from) => write!(f, "cannot convert from {:?} to {:?}", from, self.to),
            Err(from) => write!(f, "cannot convert from {:?} to {:?}", from, self.to),
        }
    }
}

impl TypeTagHash {
    pub fn new(name: &'static str) -> Self {
        let mut hash = Sha256::new();
        hash.update(name);
        let hash: [u8; 16] = hash.finalize().as_slice()[..16].try_into().unwrap();
        TypeTagHash(hash)
    }
}

impl<'a> AnyFlat<'a> {
    pub const VT_TYPE_TAG_HASH: VOffsetT = 4;
    pub const VT_DATA: VOffsetT = 6;

    #[inline]
    pub fn type_tag_hash(&self) -> Option<&'a TypeTagHash> {
        self.table.get::<TypeTagHash>(AnyFlat::VT_TYPE_TAG_HASH, None)
    }
    #[inline]
    unsafe fn data_raw<T: Follow<'a> + 'a>(&self) -> Option<T::Inner> {
        println!("{:?} {}", self.table.loc, type_name::<T>());
        self.table.get::<ForwardsUOffset<T>>(AnyFlat::VT_DATA, None)
    }
    pub fn serialize<'b, T: Serialize + HasTypeTag>(
        fbb: &'b mut FlatBufferBuilder<'a>,
        value: &T,
    ) -> crate::ser::Result<WIPOffset<AnyFlat<'a>>> {
        let data = serialize_raw(fbb, value)?;
        Ok(Self::create_raw(fbb, T::type_tag().type_tag_hash(), data))
    }
    pub fn deserialize<T: Deserialize<'a> + HasTypeTag>(&self) -> crate::de::Result<T> {
        let data = unsafe { self.follow_raw::<FollowLazy>(T::type_tag()) }?;
        deserialize_raw(data.buf, data.loc)
    }
    pub fn create<'b, T: HasTypeTag>(fbb: &'b mut FlatBufferBuilder<'a>, value: WIPOffset<T>) -> WIPOffset<AnyFlat<'a>> {
        Self::create_raw(fbb, T::type_tag().type_tag_hash(), WIPOffset::new(value.value()))
    }
    pub fn follow<T: HasTypeTag + HasFlatTypeTag + Follow<'a> + 'a>(&self) -> Result<T::Inner, TypeMismatch> {
        unsafe { self.follow_raw::<T>(T::type_tag()) }
    }
    pub unsafe fn follow_raw<T: Follow<'a> + 'a>(&self, to: &'static TypeTag) -> Result<T::Inner, TypeMismatch> {
        let from = self.type_tag_hash().unwrap();
        if from == &to.type_tag_hash() {
            Ok(self.data_raw::<T>().unwrap())
        } else {
            let from = TypeTag::lookup_hash(*from);
            Err(TypeMismatch { from, to })
        }
    }
    pub fn create_raw<'b>(
        fbb: &'b mut FlatBufferBuilder<'a>,
        type_tag_hash: TypeTagHash,
        data: WIPOffset<UnionWIPOffset>,
    ) -> WIPOffset<AnyFlat<'a>> {
        let table = fbb.start_table();
        fbb.push_slot_always(Self::VT_TYPE_TAG_HASH, &type_tag_hash);
        fbb.push_slot_always(Self::VT_DATA, data);
        WIPOffset::new(fbb.end_table(table).value())
    }
}

impl<'a> Push for &'a TypeTagHash {
    type Output = TypeTagHash;
    fn push(&self, dst: &mut [u8], _rest: &[u8]) {
        dst.copy_from_slice(&self.0)
    }
}

impl<'a> Follow<'a> for AnyFlat<'a> {
    type Inner = AnyFlat<'a>;
    #[inline]
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Self { table: Table { buf, loc } }
    }
}

impl<'a> Follow<'a> for TypeTagHash {
    type Inner = &'a TypeTagHash;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        unsafe { mem::transmute::<&'a [u8; 16], &'a TypeTagHash>(buf[loc..loc + 16].try_into().unwrap()) }
    }
}

impl Verifiable for AnyFlat<'_> {
    #[inline]
    fn run_verifier(
        v: &mut Verifier, pos: usize,
    ) -> Result<(), InvalidFlatbuffer> {
        v.visit_table(pos)?
            .visit_union::<TypeTagHash, _>(
                "type_tag_hash",
                Self::VT_TYPE_TAG_HASH,
                "data",
                Self::VT_DATA,
                true,
                |hash, value, pos| {
                    println!("Pos {:?}",pos);
                    if let Ok(tag) = TypeTag::lookup_hash(*hash) {
                        if let Some(tag_vtable) = tag.flat_type_tag() {
                            tag_vtable.run_verifier(value, pos)
                        } else {
                            Ok(())
                        }
                    } else {
                        Ok(())
                    }
                })?
            .finish();
        Ok(())
    }
}

impl<'a> Verifiable for TypeTagHash {
    fn run_verifier(v: &mut Verifier, pos: usize) -> Result<(), InvalidFlatbuffer> {
        v.in_buffer::<TypeTagHash>(pos)
    }
}

impl std::fmt::Debug for AnyFlat<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("AnyFlat");
        ds.field("type_tag_hash", &self.type_tag_hash().cloned().map(TypeTag::lookup_hash));
        ds.finish_non_exhaustive()
    }
}

impl Debug for TypeTagHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for x in self.0 {
            write!(f, "{:02X?}", x)?;
        }
        Ok(())
    }
}

registry!(
    value crate::tag::TYPE_TAGS => type_tag!(type AnyFlat<'a>, name "flatbuffer_serde::any::AnyFlat", kinds [flat]);
);
