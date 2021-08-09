use crate::ser::{AnySerializer, AnySerialize, AnySerializeSingleton};
use std::any::TypeId;
use serde::ser::Error;
use serde::Serialize;

type JsonSerializer<'b> = serde_json::Serializer<&'b mut Vec<u8>>;

impl<'a, 'b> AnySerializer for &'a mut JsonSerializer<'b> {
    fn serialize_dyn(self, value: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
        value.as_serialize_impl(TypeId::of::<dyn AnySerializeJson>())
            .ok_or(Self::Error::custom("Missing AnySerializeJson impl"))?
            .downcast_ref::<&'static dyn AnySerializeJson>()
            .ok_or(Self::Error::custom("AnySerializeJson impl wrong type"))?
            .serialize_json(self, value)
    }
}

pub trait AnySerializeJson: 'static + Send + Sync {
    fn serialize_json<'a, 'b>(&self, serializer: &'a mut JsonSerializer<'b>, value: &dyn AnySerialize) -> Result<(), serde_json::Error>;
}

impl<T: Serialize + 'static> AnySerializeJson for AnySerializeSingleton<T> {
    fn serialize_json<'a, 'b>(&self, serializer: &'a mut JsonSerializer<'b>, value: &dyn AnySerialize) -> Result<(), serde_json::Error> {
        value.as_any().downcast_ref::<T>().serialize(serializer)
    }
}