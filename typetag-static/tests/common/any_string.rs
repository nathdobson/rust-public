use std::marker::PhantomData;

use catalog::register;
use serde::{Deserialize, Serialize};
use typetag_static::impl_any_serde;

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Clone, Hash, Debug)]
pub struct AnyString(pub String);

impl_any_serde!(
    AnyString,
    "serde_any::tests::common::AnyString",
    typetag_static::json::IMPLS,
    typetag_static::binary::IMPLS
);

// registry! {
//     type typetag_static::json::IMPLS => AnyString;
//     type typetag_static::binary::IMPLS => AnyString;
// }
