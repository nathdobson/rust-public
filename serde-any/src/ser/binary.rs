use crate::binary;
use crate::ser::{AnySerializer, AnySerialize, AnySerializeSingleton};
use crate::binary::BinarySerializer;
use std::any::TypeId;
use serde::ser::{Error, SerializeTuple};
use serde::{Serialize, Serializer};
use crate::tag::HasTypeTag;

impl<'a> AnySerializer for BinarySerializer<'a> {
    fn serialize_dyn(self, value: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
        value.as_serialize_impl(TypeId::of::<dyn AnySerializeBinary>())
            .ok_or(Self::Error::MissingImpl)?
            .downcast_ref::<&'static dyn AnySerializeBinary>()
            .ok_or(Self::Error::BadImpl)?
            .serialize_binary(self, value)
    }
}

pub trait AnySerializeBinary: 'static + Send + Sync {
    fn serialize_binary<'a>(&self, serializer: BinarySerializer<'a>, value: &dyn AnySerialize) -> Result<(), binary::Error>;
}

impl<T: Serialize + 'static + HasTypeTag> AnySerializeBinary for AnySerializeSingleton<T> {
    fn serialize_binary<'a>(&self, serializer: BinarySerializer<'a>, value: &dyn AnySerialize) -> Result<(), binary::Error> {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&T::type_tag().hash)?;
        tuple.serialize_with_length(value.as_any().downcast_ref::<T>().ok_or(binary::Error::BadType)?)?;
        Ok(())
    }
}