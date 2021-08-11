use sha2::{Sha256, Digest};
use std::convert::TryInto;
use serde::Serialize;
use serde::Deserialize;

/// A globally unique hash for a TypeTag. This hash provides sufficient entropy that accidental
/// collisions are not a concern.
#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct TypeTagHash([u8; 16]);

/// A unique stable identifier for a type.
#[derive(Debug, Clone)]
pub struct TypeTag {
    pub name: &'static str,
    pub hash: TypeTagHash,
}

impl TypeTag {
    pub fn new(name: &'static str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(name);
        TypeTag {
            name,
            hash: TypeTagHash(hasher.finalize().as_slice()[0..16].try_into().expect("wrong length")),
        }
    }
}

/// A type that has an associated [`TypeTag`].
pub trait HasTypeTag {
    fn type_tag() -> &'static TypeTag;
}
