use serde::Serialize;
use serde::Deserialize;
use typetag_static::impl_any_serde;
use typetag_static::impl_any_json;
use typetag_static::impl_any_binary;

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Clone, Hash, Debug)]
pub struct AnyString(pub String);

impl_any_serde!(AnyString, "serde_any::tests::common::AnyString");
impl_any_json!(AnyString);
impl_any_binary!(AnyString);
