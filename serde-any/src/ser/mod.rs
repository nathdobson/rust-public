pub mod json;
pub mod binary;

use serde::{Serializer, Serialize};
use std::any::{Any, type_name, TypeId};
use std::ops::{DerefMut, Deref};
use bincode::{Error, Options};
use crate::tag::TypeTag;
use crate::tag::HasTypeTag;
use serde::ser::SerializeMap;
use crate::binary::BinarySerializer;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::marker::PhantomData;
use lazy_static::lazy_static;
use serde::ser::Error as _;


pub trait AnySerialize: 'static + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_serialize_impl<'a>(&self, id: TypeId) -> Option<&'static dyn Any>;
}

pub trait AnySerializer: Serializer {
    fn serialize_dyn(self, value: &dyn AnySerialize) -> Result<Self::Ok, Self::Error>;
}

impl<T: Serializer> AnySerializer for T {
    default fn serialize_dyn(self, value: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}


//
// pub(crate) trait AnySerializerImpl: Serializer {
//     fn serialize_dyn_impl(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error>;
//     fn serialize_tagged<T: Serialize + HasTypeTag>(self, value: &T) -> Result<Self::Ok, Self::Error>;
// }
//
// pub(crate) trait AnySerializer: Serializer {
//     fn serialize_dyn(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error>;
// }
//
// type JsonSerializer<'b> = serde_json::Serializer<&'b mut Vec<u8>>;
//
// #[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash, Debug)]
// pub struct AnySerializeKey(usize);
//
// static ANY_SERIALIZE_NEXT_KEY: AtomicUsize = AtomicUsize::new(0);
//
// impl AnySerializeKey {
//     pub fn new() -> Self {
//         AnySerializeKey(ANY_SERIALIZE_NEXT_KEY.fetch_add(1, Ordering::Relaxed))
//     }
// }
//
// lazy_static! {
//     pub static ref ANY_SERIALIZE_BINARY_KEY : AnySerializeKey = AnySerializeKey::new();
//     pub static ref ANY_SERIALIZE_JSON_KEY : AnySerializeKey = AnySerializeKey::new();
// }
//
// pub trait AnySerializeBinary: 'static + Send + Sync {
//     fn serialize_binary<'a>(&self, serializer: BinarySerializer<'a>, value: &dyn AnySerialize) -> Result<(), binary::Error>;
// }
//
// pub trait AnySerializeJson: 'static + Send + Sync {
//     fn serialize_json<'a, 'b>(&self, serializer: &'a mut JsonSerializer<'b>, value: &dyn AnySerialize) -> Result<(), serde_json::Error>;
// }
//
// pub struct AnySerializeEntry<T>(PhantomData<fn() -> T>);
//
// pub trait AnySerialize: 'static {
//     fn as_any(&self) -> &dyn Any;
//     fn get_any_serialize_impl(&self, key: AnySerializeKey) -> Option<&'static dyn Any>;
// }
//
// impl<T> AnySerializeEntry<T> {
//     pub const fn new() -> Self {
//         AnySerializeEntry(PhantomData)
//     }
// }
//
// impl<T: Serialize + HasTypeTag + 'static> AnySerializeBinary for &'static AnySerializeEntry<T> {
//     fn serialize_binary<'a>(&self, serializer: BinarySerializer<'a>, value: &dyn AnySerialize) -> Result<(), binary::Error> {
//         serializer.serialize_tagged(value.as_any().downcast_ref::<T>().unwrap())
//     }
// }
//
// impl<T: Serialize + HasTypeTag + 'static> AnySerializeJson for &'static AnySerializeEntry<T> {
//     fn serialize_json<'a, 'b>(&self, serializer: &'a mut JsonSerializer<'b>, value: &dyn AnySerialize) -> Result<(), serde_json::Error> {
//         serializer.serialize_tagged(value.as_any().downcast_ref::<T>().unwrap())
//     }
// }
//
// impl<'a> AnySerializerImpl for BinarySerializer<'a> {
//     fn serialize_dyn_impl(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
//         any.get_any_serialize_impl(*ANY_SERIALIZE_BINARY_KEY)
//             .ok_or(Self::Error::custom("Could not find impl"))?
//             .downcast_ref::<&'static dyn AnySerializeBinary>()
//             .ok_or(Self::Error::custom("Impl has wrong type"))?
//             .serialize_binary(self, any)
//     }
//
//     fn serialize_tagged<T: Serialize + HasTypeTag>(mut self, value: &T) -> Result<Self::Ok, Self::Error> {
//         T::type_tag().hash.serialize(self.reborrow())?;
//         self.serialize_with_length(value)?;
//         Ok(())
//     }
// }
//
// impl<'a, 'b> AnySerializerImpl for &'a mut JsonSerializer<'b> {
//     fn serialize_dyn_impl(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
//         any.get_any_serialize_impl(*ANY_SERIALIZE_JSON_KEY)
//             .ok_or(Self::Error::custom("Could not find impl"))?
//             .downcast_ref::<&'static dyn AnySerializeJson>()
//             .ok_or(Self::Error::custom("Impl has wrong type"))?
//             .serialize_json(self, any)
//     }
//
//     fn serialize_tagged<T: Serialize + HasTypeTag>(self, value: &T) -> Result<Self::Ok, Self::Error> {
//         let mut map = self.serialize_map(Some(1))?;
//         map.serialize_entry(T::type_tag().name, value)?;
//         map.end()
//     }
// }
//
// impl<S: Serializer> AnySerializer for S {
//     default fn serialize_dyn(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
//         panic!("No specialization for {:?}", type_name::<S>())
//     }
// }
//
// impl<S: AnySerializerImpl> AnySerializer for S {
//     fn serialize_dyn(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
//         self.serialize_dyn_impl(any)
//     }
// }
