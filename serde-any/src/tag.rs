use sha2::{Sha256, Digest};
use std::convert::TryInto;
use serde::Serialize;
use serde::Deserialize;

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct TypeTagHash([u8; 32]);

pub struct TypeTag {
    pub name: &'static str,
    pub hash: TypeTagHash,
}

impl TypeTag {
    pub fn new(name: &'static str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(name);
        TypeTag { name, hash: TypeTagHash(hasher.finalize().as_slice().try_into().expect("wrong length")) }
    }
}

pub trait HasTypeTag {
    fn type_tag() -> &'static TypeTag;
}


