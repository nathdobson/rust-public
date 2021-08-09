use serde::{Deserializer, Serializer, Deserialize, Serialize};
use crate::any::AnySerde;
use crate::binary::{BinarySerializer, BinaryDeserializer, Error};
use std::any::type_name;
use std::io::Cursor;
use serde_json::de::SliceRead;
use crate::binary;
use crate::tag::{TypeTag, HasTypeTag, TypeTagHash};
use std::sync::Arc;
use std::marker::PhantomData;
use lazy_static::lazy_static;
use std::collections::HashMap;
use serde::de::{Visitor, SeqAccess, DeserializeSeed, MapAccess};
use serde::de::Error as _;
use std::fmt::Formatter;
use std::fmt;
use crate::ser::AnySerialize;

pub(crate) trait AnyDeserializerImpl<'de>: Deserializer<'de> {
    fn deserialize_dyn_impl(self) -> Result<AnySerde, Self::Error>;
    fn deserialize_dyn_with_impl(self, imp: &dyn AnyDeserialize) -> Result<AnySerde, Self::Error>;
}

pub(crate) trait AnyDeserializer<'de>: Deserializer<'de> {
    fn deserialize_dyn(self) -> Result<AnySerde, Self::Error>;
    fn deserialize_dyn_with(self, imp: &dyn AnyDeserialize) -> Result<AnySerde, Self::Error>;
}

type JsonDeserializer<'de> = serde_json::Deserializer<SliceRead<'de>>;

pub trait AnyDeserialize: 'static + Send + Sync {
    fn deserialize_binary<'a, 'de>(&self, de: &'a mut BinaryDeserializer<'de>) -> Result<AnySerde, binary::Error>;
    fn deserialize_json<'a, 'de>(&self, de: &'a mut JsonDeserializer<'de>) -> Result<AnySerde, serde_json::Error>;
}

struct AnyDeserializeImpl<T>(PhantomData<fn() -> T>);

impl<T: 'static + Serialize + for<'de> Deserialize<'de> + HasTypeTag + AnySerialize> AnyDeserialize for AnyDeserializeImpl<T> {
    fn deserialize_binary<'a, 'de>(&self, de: &'a mut BinaryDeserializer<'de>) -> Result<AnySerde, Error> {
        Ok(AnySerde::new(T::deserialize(de)?))
    }
    fn deserialize_json<'a, 'de>(&self, de: &'a mut JsonDeserializer<'de>) -> Result<AnySerde, serde_json::Error> {
        Ok(AnySerde::new(T::deserialize(de)?))
    }
}

pub struct AnyDeserializeEntry {
    tag: &'static TypeTag,
    imp: Arc<dyn AnyDeserialize>,
}

inventory::collect!(AnyDeserializeEntry);

lazy_static! {
    static ref IMPL_BY_HASH: HashMap<TypeTagHash, Arc<dyn AnyDeserialize>> =
        inventory::iter::<AnyDeserializeEntry>().map(|entry|(entry.tag.hash, entry.imp.clone())).collect();
    static ref IMPL_BY_NAME: HashMap<&'static str, Arc<dyn AnyDeserialize>> =
        inventory::iter::<AnyDeserializeEntry>().map(|entry|(entry.tag.name, entry.imp.clone())).collect();
}

impl AnyDeserializeEntry {
    pub fn new<T: 'static + Serialize + for<'de> Deserialize<'de> + HasTypeTag + AnySerialize>() -> Self {
        AnyDeserializeEntry {
            tag: T::type_tag(),
            imp: Arc::new(AnyDeserializeImpl::<T>(PhantomData)),
        }
    }
}

impl<'a, 'de> AnyDeserializerImpl<'de> for &'a mut BinaryDeserializer<'de> {
    fn deserialize_dyn_impl(self) -> Result<AnySerde, Self::Error> {
        let hash: TypeTagHash = TypeTagHash::deserialize(&mut *self)?;
        let length: u64 = u64::from_le_bytes(self.read_fixed()?);
        IMPL_BY_HASH.get(&hash).unwrap_or_else(|| todo!()).deserialize_binary(self)
    }

    fn deserialize_dyn_with_impl(self, imp: &dyn AnyDeserialize) -> Result<AnySerde, Self::Error> {
        imp.deserialize_binary(self)
    }
}

impl<'a, 'de> DeserializeSeed<'de> for &dyn AnyDeserialize {
    type Value = AnySerde;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_dyn_with(self)
    }
}

impl<'a, 'de> AnyDeserializerImpl<'de> for &'a mut JsonDeserializer<'de> {
    fn deserialize_dyn_impl(self) -> Result<AnySerde, Self::Error> {
        struct Vis;
        impl<'de> Visitor<'de> for Vis {
            type Value = AnySerde;
            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                write!(formatter, "an `AnySerde` struct with fields `type` and `value`")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut seq: A) -> Result<AnySerde, A::Error> {
                let typ = seq.next_key::<&'de str>()?.ok_or(A::Error::custom("missing key"))?;
                let imp = IMPL_BY_NAME.get(typ).ok_or(A::Error::custom("unknown type"))?;
                let value_value = seq.next_value_seed(&**imp)?;
                Ok(value_value)
            }
        }
        self.deserialize_struct("AnySerde", &["type", "value"], Vis)
    }

    fn deserialize_dyn_with_impl(self, imp: &dyn AnyDeserialize) -> Result<AnySerde, Self::Error> {
        imp.deserialize_json(self)
    }
}

impl<'de, S: Deserializer<'de>> AnyDeserializer<'de> for S {
    default fn deserialize_dyn(self) -> Result<AnySerde, Self::Error> {
        panic!("No specialization for {:?}", type_name::<S>())
    }
    default fn deserialize_dyn_with(self, imp: &dyn AnyDeserialize) -> Result<AnySerde, Self::Error> {
        panic!("No specialization for {:?}", type_name::<S>())
    }
}

impl<'de, S: AnyDeserializerImpl<'de>> AnyDeserializer<'de> for S {
    fn deserialize_dyn(self) -> Result<AnySerde, Self::Error> {
        self.deserialize_dyn_impl()
    }
    fn deserialize_dyn_with(self, imp: &dyn AnyDeserialize) -> Result<AnySerde, Self::Error> {
        self.deserialize_dyn_with_impl(imp)
    }
}
