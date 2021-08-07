use serde::{Serializer, Serialize};
use std::any::{Any, type_name};
use std::ops::{DerefMut, Deref};
use bincode::Error;

pub struct AnySerde {
    inner: Box<dyn AnySerialize>,
}

trait AnySerializerInner: Serializer {
    fn serialize_any_inner(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error>;
}

trait AnySerializerOuter: Serializer {
    fn serialize_any_outer(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error>;
}

type BincodeSerializer<'a, 'b> = &'a mut bincode::Serializer<&'b mut Vec<u8>,
    bincode::config::WithOtherTrailing<
        bincode::config::WithOtherIntEncoding<
            bincode::config::DefaultOptions,
            bincode::config::FixintEncoding>,
        bincode::config::AllowTrailing>>;

type JsonSerializer<'a, 'b> = &'a mut serde_json::Serializer<&'b mut Vec<u8>>;

pub trait AnySerialize: 'static {
    fn serialize_bincode<'a, 'b>(&self, serializer: BincodeSerializer<'a, 'b>) -> Result<(), bincode::Error>;
    fn serialize_json<'a, 'b>(&self, serializer: JsonSerializer<'a, 'b>) -> Result<(), serde_json::Error>;
}

impl<T: Serialize + 'static> AnySerialize for T {
    fn serialize_bincode<'a, 'b>(&self, serializer: BincodeSerializer<'a, 'b>) -> Result<(), Error> {
        self.serialize(serializer)
    }
    fn serialize_json<'a, 'b>(&self, serializer: JsonSerializer<'a, 'b>) -> Result<(), serde_json::Error> {
        self.serialize(serializer)
    }
}

impl<'a, 'b> AnySerializerInner for BincodeSerializer<'a, 'b> {
    fn serialize_any_inner(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
        any.serialize_bincode(self)
    }
}

default impl<S: Serializer> AnySerializerOuter for S {
    fn serialize_any_outer(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
        panic!("No specialization for {:?}", type_name::<S>())
    }
}

impl<S: AnySerializerInner> AnySerializerOuter for S {
    fn serialize_any_outer(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
        self.serialize_any_inner(any)
    }
}

impl AnySerde {
    pub fn new<T: AnySerialize>(inner: T) -> Self { AnySerde { inner: Box::new(inner) } }
    pub fn into_inner(self) -> Box<dyn AnySerialize> { self.inner }
}

impl Deref for AnySerde {
    type Target = dyn AnySerialize;
    fn deref(&self) -> &Self::Target { &*self.inner }
}

impl DerefMut for AnySerde {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut *self.inner }
}
