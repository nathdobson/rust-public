use std::any::{TypeId, Any, type_name};
use serde::{ser, Deserialize};
use serde::de;
use serde::{Serialize, Deserializer};
use crate::{AnySerializerDefault, AnyDeserializer, BoxAnySerde, AnySerde};
use std::fmt;
use std::fmt::Formatter;
use serde::de::{Visitor, MapAccess, DeserializeSeed, Error};
use crate::tag::{TypeTag, HasTypeTag};
use lazy_static::lazy_static;
use std::collections::HashMap;
use serde_json::de::SliceRead;
use serde::Serializer;
use serde::ser::SerializeMap;
use std::marker::PhantomData;
use catalog::{Registry, Builder, BuilderFrom};
//use crate::util::AnySingleton;

/// A struct created by [`AnySerde`](crate::AnySerde) when deserializing a JSON value with
/// an unrecognized tag. Ensures that such values can safely be re-serialized without losing data.
#[derive(Clone, Debug)]
pub struct UnknownJson {
    tag: String,
    value: serde_json::Value,
}

type JsonSerializer<'b> = serde_json::Serializer<&'b mut Vec<u8>>;
type JsonDeserializer<'de> = serde_json::Deserializer<SliceRead<'de>>;

impl<'a, 'b> AnySerializerDefault for &'a mut JsonSerializer<'b> {
    fn serialize_dyn(self, value: &dyn AnySerde) -> Result<Self::Ok, Self::Error> {
        if let Some(unknown) = value.downcast_ref::<UnknownJson>() {
            let mut map = self.serialize_map(Some(1))?;
            map.serialize_entry(&unknown.tag, &unknown.value)?;
            map.end()?;
            Ok(())
        } else {
            IMPLS.by_type_id.get(&value.type_id())
                .ok_or(<Self::Error as ser::Error>::custom("Missing AnyJson impl"))?
                .serialize_json(self, value)
        }
    }
}

trait AnyJsonDeserializer<'de>: Deserializer<'de> {
    fn deserialize_dyn_json(self, imp: &'static dyn AnyJson) -> Result<BoxAnySerde, Self::Error>;
}

impl<'de, T: Deserializer<'de>> AnyJsonDeserializer<'de> for T {
    default fn deserialize_dyn_json(self, imp: &'static dyn AnyJson) -> Result<BoxAnySerde, Self::Error> {
        panic!("Missing impl of AnyJsonDeserializer for {}", type_name::<T>());
    }
}

impl<'a, 'de> AnyJsonDeserializer<'de> for &'a mut JsonDeserializer<'de> {
    fn deserialize_dyn_json(self, imp: &'static dyn AnyJson) -> Result<BoxAnySerde, Self::Error> {
        imp.deserialize_json(self)
    }
}

impl<'de> DeserializeSeed<'de> for &'static dyn AnyJson {
    type Value = BoxAnySerde;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_dyn_json(self)
    }
}

impl<'a, 'de> AnyDeserializer<'de> for &'a mut JsonDeserializer<'de> {
    fn deserialize_dyn_impl(self) -> Result<BoxAnySerde, Self::Error> {
        struct Vis;
        impl<'de> Visitor<'de> for Vis {
            type Value = BoxAnySerde;
            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                write!(formatter, "a map with a typetag name key and dynamic value")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut seq: A) -> Result<BoxAnySerde, A::Error> {
                let typ = seq.next_key::<&'de str>()?.ok_or(<A::Error as de::Error>::custom("missing key"))?;
                if let Some(imp) = IMPLS.by_type_tag_name.get(typ) {
                    seq.next_value_seed(&**imp)
                } else {
                    Ok(Box::new(UnknownJson {
                        tag: typ.to_string(),
                        value: seq.next_value::<serde_json::Value>()?,
                    }))
                }
            }
        }
        self.deserialize_map(Vis)
    }
}

pub trait AnyJson: 'static + Send + Sync {
    fn inner_type_tag(&self) -> &'static TypeTag;
    fn inner_type_id(&self) -> TypeId;
    fn serialize_json<'a, 'b>(&self, serializer: &'a mut JsonSerializer<'b>, value: &dyn AnySerde) -> Result<(), serde_json::Error>;
    fn deserialize_json<'a, 'de>(&self, deserializer: &'a mut JsonDeserializer<'de>) -> Result<BoxAnySerde, serde_json::Error>;
}

impl<T: Serialize + for<'de> Deserialize<'de> + 'static + HasTypeTag + AnySerde> AnyJson for PhantomData<T> {
    fn inner_type_tag(&self) -> &'static TypeTag { T::type_tag() }
    fn inner_type_id(&self) -> TypeId { TypeId::of::<T>() }
    fn serialize_json<'a, 'b>(&self, serializer: &'a mut JsonSerializer<'b>, value: &dyn AnySerde) -> Result<(), serde_json::Error> {
        let mut struc = serializer.serialize_map(Some(1))?;
        let value = value.downcast_ref::<T>().ok_or(<serde_json::Error as ser::Error>::custom("Bad type passed to AnyJson"))?;
        struc.serialize_entry(self.inner_type_tag().name, value)?;
        struc.end()?;
        Ok(())
    }
    fn deserialize_json<'a, 'de>(&self, deserializer: &'a mut JsonDeserializer<'de>) -> Result<BoxAnySerde, serde_json::Error> {
        Ok(Box::new(T::deserialize(deserializer)?))
    }
}

pub fn serialize<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

pub fn serialize_into<T: Serialize>(output: &mut Vec<u8>, value: &T) -> Result<(), serde_json::Error> {
    value.serialize(&mut JsonSerializer::new(output))?;
    Ok(())
}

pub fn deserialize<'de, T: Deserialize<'de>>(slice: &'de [u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(slice)
}

impl AnySerde for UnknownJson {
    fn clone_box(&self) -> BoxAnySerde {
        Box::new(self.clone())
    }

    fn inner_type_name(&self) -> &'static str {
        "typetag_static::json::UnknownJson"
    }
}

pub struct Impls {
    by_type_id: HashMap<TypeId, &'static dyn AnyJson>,
    by_type_tag_name: HashMap<&'static str, &'static dyn AnyJson>,
}

impl Builder for Impls {
    type Output = Self;

    fn new() -> Self {
        Impls {
            by_type_id: HashMap::new(),
            by_type_tag_name: HashMap::new(),
        }
    }

    fn build(self) -> Self::Output { self }
}

impl BuilderFrom<&'static dyn AnyJson> for Impls {
    fn insert(&mut self, element: &'static dyn AnyJson) {
        self.by_type_id.insert(element.inner_type_id(), element);
        self.by_type_tag_name.insert(element.inner_type_tag().name, element);
    }
}

pub static IMPLS: Registry<Impls> = Registry::new();
