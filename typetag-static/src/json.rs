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
use crate::util::AnySingleton;

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
            IMPL_BY_TYPE_ID.get(&value.type_id())
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
                if let Some(imp) = IMPL_BY_TYPE_TAG_NAME.get(typ) {
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

impl<T: Serialize + for<'de> Deserialize<'de> + 'static + HasTypeTag + AnySerde> AnyJson for AnySingleton<T> {
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

pub fn deserialize<'de, T: Deserialize<'de>>(slice: &'de [u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(slice)
}

impl AnySerde for UnknownJson {
    fn clone_box(&self) -> BoxAnySerde {
        Box::new(self.clone())
    }
}

inventory::collect!(&'static dyn AnyJson);

lazy_static! {
    static ref IMPL_BY_TYPE_ID: HashMap<TypeId, &'static dyn AnyJson> =
        inventory::iter::<&'static dyn AnyJson>().map(|x| (x.inner_type_id(), *x)).collect();
    static ref IMPL_BY_TYPE_TAG_NAME: HashMap<&'static str, &'static dyn AnyJson> =
        inventory::iter::<&'static dyn AnyJson>().map(|x| (x.inner_type_tag().name, *x)).collect();
}
