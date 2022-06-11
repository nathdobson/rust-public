use std::any::{Any, TypeId};
use std::collections::HashMap;
//use crate::util::AnySingleton;
use std::fmt::{Debug, Formatter};
use std::io::Seek;
use std::marker::PhantomData;
use std::sync::Arc;

use catalog::{Builder, BuilderFrom, Registry};
use lazy_static::lazy_static;
use serde::ser::SerializeTuple;
use serde::{de, ser, Deserialize, Serialize, Serializer};

use crate::binary::de::BinaryDeserializer;
use crate::binary::ser::BinarySerializer;
use crate::binary::{Error, UnknownBinary};
use crate::tag::{HasTypeTag, TypeTag, TypeTagHash};
use crate::{AnyDeserializer, AnySerde, AnySerializerDefault, ArcAnySerde, BoxAnySerde};

impl<'a> AnySerializerDefault for BinarySerializer<'a> {
    fn serialize_dyn(mut self, value: &dyn AnySerde) -> Result<Self::Ok, Self::Error> {
        if let Some(unknown) = value.downcast_ref::<UnknownBinary>() {
            unknown.tag.serialize(self.reborrow())?;
            self.reborrow()
                .serialize_u64(unknown.content.len() as u64)?;
            self.serialize_raw(&unknown.content)?;
            Ok(())
        } else {
            let id = value.type_id();
            IMPLS
                .by_type_id
                .get(&id)
                .ok_or(Error::MissingSerialize(value.inner_type_name().to_string()))?
                .serialize_binary(self, value)
        }
    }
}

impl Debug for dyn AnyBinary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnyBinary<{:?}>", self.inner_type_tag())
    }
}

impl<'a, 'de> AnyDeserializer<'de> for &'a mut BinaryDeserializer<'de> {
    fn deserialize_box_impl(self) -> Result<BoxAnySerde, Self::Error> {
        let tag: TypeTagHash = TypeTagHash::deserialize(&mut *self)?;
        let length = u64::from_le_bytes(self.read_fixed()?);
        if let Some(imp) = IMPLS.by_type_tag_hash.get(&tag) {
            imp.deserialize_box_binary(self)
        } else {
            if length > self.cursor.stream_len().unwrap() {
                return Err(Self::Error::BadLength);
            }
            let mut content = vec![0; length as usize];
            self.read_exact(&mut content)?;
            Ok(Box::new(UnknownBinary { tag, content }))
        }
    }

    fn deserialize_arc_impl(self) -> Result<ArcAnySerde, Self::Error> {
        let tag: TypeTagHash = TypeTagHash::deserialize(&mut *self)?;
        let length = u64::from_le_bytes(self.read_fixed()?);
        if let Some(imp) = IMPLS.by_type_tag_hash.get(&tag) {
            imp.deserialize_arc_binary(self)
        } else {
            if length > self.cursor.stream_len().unwrap() {
                return Err(Self::Error::BadLength);
            }
            let mut content = vec![0; length as usize];
            self.read_exact(&mut content)?;
            Ok(ArcAnySerde::new(UnknownBinary { tag, content }))
        }
    }
}

pub trait AnyBinary: 'static + Send + Sync {
    fn inner_type_tag(&self) -> &'static TypeTag;
    fn inner_type_id(&self) -> TypeId;
    fn serialize_binary<'a>(
        &self,
        serializer: BinarySerializer<'a>,
        value: &dyn AnySerde,
    ) -> Result<(), Error>;
    fn deserialize_box_binary<'a, 'de>(
        &self,
        deserializer: &'a mut BinaryDeserializer<'de>,
    ) -> Result<BoxAnySerde, Error>;
    fn deserialize_arc_binary<'a, 'de>(
        &self,
        deserializer: &'a mut BinaryDeserializer<'de>,
    ) -> Result<ArcAnySerde, Error>;
}

impl<T: Serialize + for<'de> Deserialize<'de> + 'static + HasTypeTag + AnySerde> AnyBinary
    for PhantomData<T>
{
    fn inner_type_tag(&self) -> &'static TypeTag { T::type_tag() }
    fn inner_type_id(&self) -> TypeId { TypeId::of::<T>() }
    fn serialize_binary<'a>(
        &self,
        mut serializer: BinarySerializer<'a>,
        value: &dyn AnySerde,
    ) -> Result<(), Error> {
        T::type_tag().hash.serialize(serializer.reborrow())?;
        serializer.serialize_with_length(value.downcast_ref::<T>().ok_or(Error::BadType)?)?;
        Ok(())
    }
    fn deserialize_box_binary<'a, 'de>(
        &self,
        deserializer: &'a mut BinaryDeserializer<'de>,
    ) -> Result<BoxAnySerde, Error> {
        Ok(Box::new(T::deserialize(deserializer)?))
    }
    fn deserialize_arc_binary<'a, 'de>(
        &self,
        deserializer: &'a mut BinaryDeserializer<'de>,
    ) -> Result<ArcAnySerde, Error> {
        Ok(ArcAnySerde::new(T::deserialize(deserializer)?))
    }
}

impl AnySerde for UnknownBinary {
    fn clone_box(&self) -> BoxAnySerde { Box::new(self.clone()) }

    fn inner_type_name(&self) -> &'static str { "typetag_static::binary::UnknownBinary" }
}

pub struct Impls {
    by_type_id: HashMap<TypeId, &'static dyn AnyBinary>,
    by_type_tag_hash: HashMap<TypeTagHash, &'static dyn AnyBinary>,
}

impl Builder for Impls {
    type Output = Self;
    fn new() -> Self {
        Impls {
            by_type_id: HashMap::new(),
            by_type_tag_hash: HashMap::new(),
        }
    }
    fn build(self) -> Self::Output { self }
}

impl BuilderFrom<&'static dyn AnyBinary> for Impls {
    fn insert(&mut self, element: &'static dyn AnyBinary) {
        assert!(self
            .by_type_id
            .insert(element.inner_type_id(), element)
            .is_none());
        assert!(self
            .by_type_tag_hash
            .insert(element.inner_type_tag().hash, element)
            .is_none());
    }
}

pub static IMPLS: Registry<Impls> = Registry::new();
