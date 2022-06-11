use std::fmt::{Debug, Formatter};

use flatbuffers::{FlatBufferBuilder, UOffsetT, UnionWIPOffset, VOffsetT, WIPOffset};

use crate::flat_util::{Flat128, FlatUnit, VariantT};
use crate::ser::wrapper::Serializer;

#[derive(Copy, Clone)]
pub enum OneValue {
    Ref(WIPOffset<UnionWIPOffset>),
    SomeRef(WIPOffset<UnionWIPOffset>),
    NoneRef,
    Fixed0,
    Fixed8(u8),
    Fixed16(u16),
    Fixed32(u32),
    Fixed64(u64),
    Fixed128(u128),
}

#[derive(Copy, Clone, Debug)]
pub enum Value {
    OneValue(OneValue),
    Enum { variant: VariantT, value: OneValue },
}

impl Debug for OneValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OneValue::Ref(x) => f.debug_tuple("OneValue::Ref").field(&x.value()).finish(),
            OneValue::SomeRef(x) => f
                .debug_tuple("OneValue::SomeRef")
                .field(&x.value())
                .finish(),
            OneValue::NoneRef => f.debug_struct("OneValue::NoneRef").finish(),
            OneValue::Fixed0 => f.debug_struct("OneValue::Fixed0").finish(),
            OneValue::Fixed8(x) => f.debug_tuple("OneValue::Fixed8").field(&x).finish(),
            OneValue::Fixed16(x) => f.debug_tuple("OneValue::Fixed16").field(&x).finish(),
            OneValue::Fixed32(x) => f.debug_tuple("OneValue::Fixed32").field(&x).finish(),
            OneValue::Fixed64(x) => f.debug_tuple("OneValue::Fixed64").field(&x).finish(),
            OneValue::Fixed128(x) => f.debug_tuple("OneValue::Fixed128").field(&x).finish(),
        }
    }
}

impl Value {
    pub fn to_one_value(self, serializer: &mut Serializer) -> OneValue {
        match self {
            Value::OneValue(x) => x,
            Value::Enum { variant, value } => {
                let mut builder = serializer.reborrow().start_table();
                builder.push(Value::Enum { variant, value });
                OneValue::Ref(builder.end_table())
            }
        }
    }

    pub fn to_offset(self, serializer: &mut Serializer) -> WIPOffset<UnionWIPOffset> {
        let value = self.to_one_value(serializer);
        let value = value.to_offset(serializer.fbb, value);
        value
    }
}

impl OneValue {
    pub fn push_slot_always(self, fbb: &mut FlatBufferBuilder, off: VOffsetT) {
        match self {
            OneValue::Ref(x) => fbb.push_slot_always(off, x),
            OneValue::SomeRef(x) => fbb.push_slot_always(off, x),
            OneValue::NoneRef => {}
            OneValue::Fixed0 => {}
            OneValue::Fixed8(x) => fbb.push_slot(off, x, 0),
            OneValue::Fixed16(x) => fbb.push_slot(off, x, 0),
            OneValue::Fixed32(x) => fbb.push_slot(off, x, 0),
            OneValue::Fixed64(x) => fbb.push_slot(off, x, 0),
            OneValue::Fixed128(x) => fbb.push_slot(off, Flat128(x), Flat128(0)),
        }
    }

    pub fn to_offset(
        self,
        fbb: &mut FlatBufferBuilder,
        value: OneValue,
    ) -> WIPOffset<UnionWIPOffset> {
        match self {
            OneValue::Ref(x) => x,
            OneValue::SomeRef(x) => fbb.push(x).as_union_value(),
            OneValue::NoneRef => fbb.push(0 as UOffsetT).as_union_value(),
            OneValue::Fixed0 => fbb.push(FlatUnit).as_union_value(),
            OneValue::Fixed8(x) => fbb.push(x).as_union_value(),
            OneValue::Fixed16(x) => fbb.push(x).as_union_value(),
            OneValue::Fixed32(x) => fbb.push(x).as_union_value(),
            OneValue::Fixed64(x) => fbb.push(x).as_union_value(),
            OneValue::Fixed128(x) => fbb.push(Flat128(x)).as_union_value(),
        }
    }
}
