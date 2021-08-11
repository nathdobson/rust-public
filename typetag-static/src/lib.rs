#![feature(specialization, never_type, const_fn_fn_ptr_basics)]
#![feature(coerce_unsized)]
#![allow(incomplete_features, unused_variables, dead_code, unused_imports, unused_macros, unused_mut)]
#![deny(unused_must_use)]

//! A crate that allows `Box<dyn Any>` to be serialized and deserialized using [`serde`].
//! ```
//! # use serde::{Serialize, Deserialize};
//! use typetag_static::{json, impl_any_serde, impl_any_json, BoxAnySerde};
//!
//! // Implement a normal struct with serde support.
//! #[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
//! pub struct MyStruct { foo: u32 }
//!
//! // Give a stable globally unique name to identify MyStruct.
//! impl_any_serde!(MyStruct, "typetag_static::docs::MyStruct");
//! // Register an implementation for MyStruct that supports JSON.
//! impl_any_json!(MyStruct);
//!
//! let input: BoxAnySerde = Box::new(MyStruct { foo: 10 });
//! let encoded = json::serialize(&input).unwrap();
//! assert_eq!(r#"{"typetag_static::docs::MyStruct":{"foo":10}}"#, encoded);
//! let output: BoxAnySerde = json::deserialize::<BoxAnySerde>(encoded.as_bytes()).unwrap();
//! assert_eq!(10, output.downcast_ref::<MyStruct>().unwrap().foo);
//! ```

use std::marker::PhantomData;
use std::any::{Any, type_name, TypeId};
use std::ops::{Deref, DerefMut, CoerceUnsized};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use std::borrow::{Borrow, BorrowMut};
use std::sync::Arc;

#[macro_use]
mod macros;
/// Support for assigning stable identifiers to types.
pub mod tag;
/// Support for JSON encoding.
pub mod json;
/// A serialization format similar to [`bincode`](https://crates.io/crates/bincode) that supports [`AnySerde`](crate::AnySerde).
pub mod binary;
#[doc(hidden)]
pub mod util;
#[doc(hidden)]
pub mod reexport;
mod impls;

pub trait TraitAnySerde: Any + Send + Sync + 'static {
    fn clone_box(&self) -> BoxAnySerde;
}

pub fn downcast_box<T: TraitAnySerde>(b: Box<dyn TraitAnySerde>) -> Result<Box<T>, Box<dyn TraitAnySerde>> {
    if b.deref().is::<T>() {
        unsafe {
            let raw: *mut dyn TraitAnySerde = Box::into_raw(b);
            Ok(Box::from_raw(raw as *mut T))
        }
    } else {
        Err(b)
    }
}

/// A wrapper around [`Box<dyn Any>`] that implements [`Serialize`] and [`Deserialize`].

pub type BoxAnySerde = Box<dyn TraitAnySerde>;

impl dyn TraitAnySerde {
    pub fn is<T: Any>(&self) -> bool {
        let t = TypeId::of::<T>();
        let concrete = self.type_id();
        t == concrete
    }
}

impl dyn TraitAnySerde {
    pub fn downcast_ref<T: TraitAnySerde>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const dyn TraitAnySerde as *const T)) }
        } else {
            None
        }
    }
    pub fn downcast_mut<T: TraitAnySerde>(&mut self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const dyn TraitAnySerde as *const T)) }
        } else {
            None
        }
    }
}

impl Serialize for BoxAnySerde {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_dyn(&**self)
    }
}

impl Serialize for &dyn TraitAnySerde {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_dyn(*self)
    }
}

impl Clone for BoxAnySerde {
    fn clone(&self) -> Self { self.clone_box() }
}

impl<'de> Deserialize<'de> for BoxAnySerde {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_dyn()
    }
}

pub(crate) trait AnyDeserializerDefault<'de>: Deserializer<'de> {
    fn deserialize_dyn(self) -> Result<BoxAnySerde, Self::Error>;
}

/// A trait that extends [`Deserializer`] with the ability to produce [`AnySerde`].
pub trait AnyDeserializer<'de>: Deserializer<'de> {
    fn deserialize_dyn_impl(self) -> Result<BoxAnySerde, Self::Error>;
}

impl<'de, D: Deserializer<'de>> AnyDeserializerDefault<'de> for D {
    default fn deserialize_dyn(self) -> Result<BoxAnySerde, D::Error> {
        panic!("Missing AnyDeserializerImpl impl for {}", type_name::<D>());
    }
}

impl<'de, D: AnyDeserializer<'de>> AnyDeserializerDefault<'de> for D {
    fn deserialize_dyn(self) -> Result<BoxAnySerde, D::Error> {
        self.deserialize_dyn_impl()
    }
}

pub(crate) trait AnySerializerDefault: Serializer {
    fn serialize_dyn(self, value: &dyn TraitAnySerde) -> Result<Self::Ok, Self::Error>;
}

/// A trait that extends [`Serializer`] with the ability to consume `&dyn Any`.
pub trait AnySerializer: Serializer {
    fn serialize_dyn_impl(self, value: &dyn TraitAnySerde) -> Result<Self::Ok, Self::Error>;
}

impl<T: Serializer> AnySerializerDefault for T {
    default fn serialize_dyn(self, value: &dyn TraitAnySerde) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<T: AnySerializer> AnySerializerDefault for T {
    fn serialize_dyn(self, value: &dyn TraitAnySerde) -> Result<Self::Ok, Self::Error> {
        self.serialize_dyn_impl(value)
    }
}

