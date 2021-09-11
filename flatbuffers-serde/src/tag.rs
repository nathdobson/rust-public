use std::collections::HashMap;
use std::any::{TypeId, type_name};
use registry::{registry, Registry, BuilderFrom};
use sha2::{Sha256, Digest};
use std::convert::TryInto;
use std::fmt::{Debug, Formatter};
use lazy_static::lazy_static;
use std::marker::PhantomData;
use crate::any_generated::{TypeTagHash};
use flatbuffers::{Verifier, InvalidFlatbuffer, Verifiable, WIPOffset, ForwardsUOffset};

#[derive(Debug)]
pub struct TypeTag {
    type_tag_hash: TypeTagHash,
    type_tag_name: &'static str,
    native_name: &'static str,
    native_id: TypeId,
    flat_type_tag: Option<FlatTypeTag>,
}

#[derive(Copy, Clone)]
pub struct FlatTypeTag {
    run_verifier: for<'a, 'b, 'c> fn(&'a mut Verifier<'b, 'c>, loc: usize) -> Result<(), InvalidFlatbuffer>,
}

pub struct TypeTagSet {
    by_native_id: HashMap<TypeId, &'static TypeTag>,
    by_hash: HashMap<TypeTagHash, &'static TypeTag>,
    by_name: HashMap<&'static str, &'static TypeTag>,
}


pub trait HasTypeTag {
    fn type_tag() -> &'static TypeTag;
}

pub unsafe trait HasFlatTypeTag {
    fn flat_type_tag() -> &'static FlatTypeTag;
}

pub static TYPE_TAGS: Registry<TypeTagSet> = Registry::new();

impl TypeTag {
    pub fn type_tag_hash(&self) -> TypeTagHash { self.type_tag_hash }
    pub fn type_tag_name(&self) -> &'static str { self.type_tag_name }
    pub fn native_name(&self) -> &'static str { self.native_name }
    pub fn native_id(&self) -> TypeId { self.native_id }
    pub fn flat_type_tag(&self) -> Option<FlatTypeTag> { self.flat_type_tag }
}

impl FlatTypeTag {
    pub fn run_verifier(&self, verifier: &mut Verifier, loc: usize) -> Result<(), InvalidFlatbuffer> {
        (self.run_verifier)(verifier, loc)
    }
}

impl registry::Builder for TypeTagSet {
    type Output = TypeTagSet;
    fn new() -> Self {
        TypeTagSet {
            by_native_id: HashMap::new(),
            by_hash: HashMap::new(),
            by_name: HashMap::new(),
        }
    }
    fn build(self) -> Self::Output { self }
}

impl<T: HasTypeTag> BuilderFrom<PhantomData<T>> for TypeTagSet {
    fn insert(&mut self, _: PhantomData<T>) {
        let tag = T::type_tag();
        self.by_native_id.insert(tag.native_id, tag).map(|other| {
            panic!("Collision of TypeId for {:?} and {:?}", tag, other);
        });
        self.by_hash.insert(tag.type_tag_hash, tag).map(|other| {
            panic!("Collision of TypeTagHash for {:?} and {:?}", tag, other);
        });
        self.by_name.insert(tag.type_tag_name, tag).map(|other| {
            panic!("Collision of name for {:?} and {:?}", tag, other);
        });
    }
}

impl TypeTag {
    pub fn new<T: 'static>(name: &'static str, flat_type_tag: Option<FlatTypeTag>) -> Self {
        TypeTag {
            type_tag_hash: TypeTagHash::new(name),
            type_tag_name: name,
            native_name: type_name::<T>(),
            native_id: TypeId::of::<T>(),
            flat_type_tag,
        }
    }
    pub fn lookup_hash(hash: TypeTagHash) -> Result<&'static Self, TypeTagHash> {
        TYPE_TAGS.by_hash.get(&hash).cloned().ok_or(hash)
    }
}

impl FlatTypeTag {
    pub const fn new<T: 'static + Verifiable>() -> Self {
        FlatTypeTag { run_verifier: ForwardsUOffset::<T>::run_verifier }
    }
}

unsafe impl<T: HasFlatTypeTag> HasFlatTypeTag for WIPOffset<T> {
    fn flat_type_tag() -> &'static FlatTypeTag {
        T::flat_type_tag()
    }
}

registry! {
    value TYPE_TAGS => type_tag!(type u8, name "std::u8", kinds [flat, serde]);
    value TYPE_TAGS => type_tag!(type u16, name "std::u16", kinds [flat, serde]);
    value TYPE_TAGS => type_tag!(type u32, name "std::u32", kinds [flat, serde]);
    value TYPE_TAGS => type_tag!(type u64, name "std::u64", kinds [flat, serde]);
    value TYPE_TAGS => type_tag!(type u128, name "std::u128", kinds [serde]);
    value TYPE_TAGS => type_tag!(type i8, name "std::i8", kinds [flat, serde]);
    value TYPE_TAGS => type_tag!(type i16, name "std::i16", kinds [flat, serde]);
    value TYPE_TAGS => type_tag!(type i32, name "std::i32", kinds [flat, serde]);
    value TYPE_TAGS => type_tag!(type i64, name "std::i64", kinds [flat, serde]);
    value TYPE_TAGS => type_tag!(type i128, name "std::i128", kinds [serde]);
    value TYPE_TAGS => type_tag!(type String, name "std::string::String", kinds [serde]);
}

impl Debug for TypeTagSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.by_hash.values()).finish()
    }
}

impl Debug for FlatTypeTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlatTypeTag")
            .field("run_verifier", &())
            .finish()
    }
}
