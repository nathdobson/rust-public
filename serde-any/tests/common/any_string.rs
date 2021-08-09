use serde::Serialize;
use serde::Deserialize;
use serde_any::ser::binary::AnySerializeBinary;
use serde_any::ser::json::AnySerializeJson;
use serde_any::ser::{AnySerialize, AnySerializeSingleton};
use serde_any::binary::BinarySerializer;
use serde_any::binary;
use std::any::{Any, TypeId};
use crate::common::custom::AnySerializeCustom;
use lazy_static::lazy_static;
use serde_any::tag::{TypeTag, HasTypeTag};

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Clone, Hash, Debug)]
pub struct AnyString(pub String);

lazy_static! {
    pub static ref ANY_STRING_TYPE_TAG: TypeTag = TypeTag::new("serde_any::tests::common::AnyString");
}

impl HasTypeTag for AnyString {
    fn type_tag() -> &'static TypeTag { &*ANY_STRING_TYPE_TAG }
}

static ANY32_IMPL: AnySerializeSingleton<AnyString> = AnySerializeSingleton::new();
static ANY32_IMPL_BINARY: &'static dyn AnySerializeBinary = &ANY32_IMPL;
static ANY32_IMPL_JSON: &'static dyn AnySerializeJson = &ANY32_IMPL;
static ANY32_IMPL_CUSTOM: &'static dyn AnySerializeCustom = &ANY32_IMPL;

impl AnySerialize for AnyString {
    fn as_any(&self) -> &dyn Any { self }
    fn as_serialize_impl(&self, id: TypeId) -> Option<&'static dyn Any> {
        if id == TypeId::of::<dyn AnySerializeBinary>() {
            Some(&ANY32_IMPL_BINARY)
        } else if id == TypeId::of::<dyn AnySerializeJson>() {
            Some(&ANY32_IMPL_JSON)
        } else if id == TypeId::of::<dyn AnySerializeCustom>() {
            Some(&ANY32_IMPL_CUSTOM)
        } else {
            None
        }
    }
}