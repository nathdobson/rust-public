use serde::Serialize;
use serde::Deserialize;
use serde_any::ser::binary::AnySerializeBinary;
use serde_any::ser::AnySerialize;
use serde_any::binary::BinarySerializer;
use serde_any::binary;
use std::any::{Any, TypeId};

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Hash, Debug)]
struct Any32(i32);

struct Any32Impl;

impl AnySerializeBinary for Any32Impl {
    fn serialize_binary<'a>(&self, serializer: BinarySerializer<'a>, value: &dyn AnySerialize) -> Result<(), binary::Error> {
        value.as_any().downcast_ref::<Any32>().serialize(serializer)
    }
}

static ANY32_IMPL: Any32Impl = Any32Impl;
static ANY32_IMPL_REF: &'static dyn AnySerializeBinary = &ANY32_IMPL;

impl AnySerialize for Any32 {
    fn as_any(&self) -> &dyn Any { self }
    fn as_serialize_impl(&self, id: TypeId) -> Option<&'static dyn Any> {
        if id == TypeId::of::<dyn AnySerializeBinary>() {
            Some(&ANY32_IMPL_REF)
        } else {
            None
        }
    }
}