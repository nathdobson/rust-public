use serde::Serialize;
use serde::Deserialize;
use typetag_static::impl_any_serde;
use registry::registry;
use std::marker::PhantomData;

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Clone, Hash, Debug)]
pub struct AnyString(pub String);

impl_any_serde!(AnyString, "serde_any::tests::common::AnyString");

registry!{
    type typetag_static::json::IMPLS => AnyString;
    type typetag_static::binary::IMPLS => AnyString;
}