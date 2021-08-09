use crate::ser::{AnySerialize, AnySerializer};
use std::ops::{Deref, DerefMut};
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use crate::de::AnyDeserializer;

pub struct AnySerde {
    inner: Box<dyn AnySerialize>,
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

impl Serialize for AnySerde {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_dyn(&*self.inner)
    }
}

impl<'de> Deserialize<'de> for AnySerde {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_dyn()
    }
}