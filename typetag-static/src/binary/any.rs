use crate::binary::{Error, UnknownBinary};
use crate::{AnySerializerDefault, AnyDeserializer, BoxAnySerde, AnySerde};
use serde::{Deserialize, Serialize, Serializer};
use std::any::Any;
use crate::binary::de::BinaryDeserializer;
use crate::tag::{TypeTagHash, HasTypeTag, TypeTag};
use crate::binary::ser::BinarySerializer;
use serde::ser::SerializeTuple;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::any::TypeId;
use serde::ser;
use serde::de;
use crate::util::AnySingleton;
use std::fmt::{Debug, Formatter};

impl<'a> AnySerializerDefault for BinarySerializer<'a> {
    fn serialize_dyn(mut self, value: &dyn AnySerde) -> Result<Self::Ok, Self::Error> {
        if let Some(unknown) = value.downcast_ref::<UnknownBinary>() {
            unknown.tag.serialize(self.reborrow())?;
            self.reborrow().serialize_u64(unknown.content.len() as u64)?;
            self.serialize_raw(&unknown.content)?;
            Ok(())
        } else {
            let id = value.type_id();
            IMPL_BY_TYPE_ID.get(&id)
                .ok_or(Error::MissingSerialize(id))?
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
    fn deserialize_dyn_impl(self) -> Result<BoxAnySerde, Self::Error> {
        let tag: TypeTagHash = TypeTagHash::deserialize(&mut *self)?;
        let length = u64::from_le_bytes(self.read_fixed()?);
        if let Some(imp) = IMPL_BY_TYPE_TAG_HASH.get(&tag) {
            imp.deserialize_binary(self)
        } else {
            let mut content = vec![0; length as usize];
            self.read_exact(&mut content)?;
            Ok(Box::new(UnknownBinary { tag, content }))
        }
    }
}

pub trait AnyBinary: 'static + Send + Sync {
    fn inner_type_tag(&self) -> &'static TypeTag;
    fn inner_type_id(&self) -> TypeId;
    fn serialize_binary<'a>(&self, serializer: BinarySerializer<'a>, value: &dyn AnySerde) -> Result<(), Error>;
    fn deserialize_binary<'a, 'de>(&self, deserializer: &'a mut BinaryDeserializer<'de>) -> Result<BoxAnySerde, Error>;
}

impl<T: Serialize + for<'de> Deserialize<'de> + 'static + HasTypeTag + AnySerde> AnyBinary for AnySingleton<T> {
    fn inner_type_tag(&self) -> &'static TypeTag { T::type_tag() }
    fn inner_type_id(&self) -> TypeId { TypeId::of::<T>() }
    fn serialize_binary<'a>(&self, mut serializer: BinarySerializer<'a>, value: &dyn AnySerde) -> Result<(), Error> {
        T::type_tag().hash.serialize(serializer.reborrow())?;
        serializer.serialize_with_length(value.downcast_ref::<T>().ok_or(Error::BadType)?)?;
        Ok(())
    }
    fn deserialize_binary<'a, 'de>(&self, deserializer: &'a mut BinaryDeserializer<'de>) -> Result<BoxAnySerde, Error> {
        Ok(Box::new(T::deserialize(deserializer)?))
    }
}

impl AnySerde for UnknownBinary {
    fn clone_box(&self) -> BoxAnySerde {
        Box::new(self.clone())
    }
}

inventory::collect!(&'static dyn AnyBinary);

lazy_static! {
    static ref IMPL_BY_TYPE_ID: HashMap<TypeId, &'static dyn AnyBinary> =
        inventory::iter::<&'static dyn AnyBinary>().map(|x| (x.inner_type_id(), *x)).collect();
    static ref IMPL_BY_TYPE_TAG_HASH: HashMap<TypeTagHash, &'static dyn AnyBinary> =
        inventory::iter::<&'static dyn AnyBinary>().map(|x| (x.inner_type_tag().hash, *x)).collect();
}
