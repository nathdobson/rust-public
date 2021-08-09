use crate::binary;
use crate::ser::{AnySerializer, AnySerialize};
use crate::binary::BinarySerializer;
use std::any::TypeId;
use serde::ser::Error;

impl<'a> AnySerializer for BinarySerializer<'a> {
    fn serialize_dyn(self, value: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
        value.as_serialize_impl(TypeId::of::<dyn AnySerializeBinary>())
            .ok_or(Self::Error::custom("Missing AnySerializeBinary impl"))?
            .downcast_ref::<&'static dyn AnySerializeBinary>()
            .ok_or(Self::Error::custom("AnySerializeBinary impl wrong type"))?
            .serialize_binary(self, value)
    }
}

pub trait AnySerializeBinary: 'static + Send + Sync {
    fn serialize_binary<'a>(&self, serializer: BinarySerializer<'a>, value: &dyn AnySerialize) -> Result<(), binary::Error>;
}