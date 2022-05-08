#![feature(specialization, never_type)]
#![feature(coerce_unsized)]
#![feature(seek_stream_len)]
#![allow(incomplete_features, unused_variables, dead_code, unused_imports, unused_macros, unused_mut)]
#![deny(unused_must_use)]
#![feature(once_cell)]

//! A crate that allows `Box<dyn Any>` to be serialized and deserialized using [`serde`].
//! ```
//! # #![feature(once_cell)]
//! # use serde::{Serialize, Deserialize};
//! # use std::marker::PhantomData;
//! use typetag_static::impl_any_serde;
//! use typetag_static::{json, BoxAnySerde};
//! use catalog::register;
//!
//! // Implement a normal struct with serde support.
//! #[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
//! pub struct MyStruct { foo: u32 }
//!
//! // Give a stable globally unique name to identify MyStruct.
//! impl_any_serde!(MyStruct, "typetag_static::docs::MyStruct", typetag_static::json::IMPLS);
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
use std::fmt::Debug;
use catalog::register;

#[macro_use]
mod macros;
/// Support for assigning stable identifiers to types.
pub mod tag;
/// Support for JSON encoding.
pub mod json;
/// A serialization format similar to [`bincode`](https://crates.io/crates/bincode) that supports [`AnySerde`](crate::AnySerde).
pub mod binary;

// #[doc(hidden)]
// pub mod util;
#[doc(hidden)]
pub mod reexport {
    pub use catalog;
}

mod impls;

pub trait AnySerde: Any + Send + Sync + Debug + 'static {
    fn clone_box(&self) -> BoxAnySerde;
    fn inner_type_name(&self) -> &'static str;
}

pub fn downcast_box<T: AnySerde>(b: Box<dyn AnySerde>) -> Result<Box<T>, Box<dyn AnySerde>> {
    if b.deref().is::<T>() {
        unsafe {
            let raw: *mut dyn AnySerde = Box::into_raw(b);
            Ok(Box::from_raw(raw as *mut T))
        }
    } else {
        Err(b)
    }
}

pub fn downcast_arc<T: AnySerde>(b: Arc<dyn AnySerde>) -> Result<Arc<T>, Arc<dyn AnySerde>> {
    if b.deref().is::<T>() {
        unsafe {
            let raw: *const dyn AnySerde = Arc::into_raw(b);
            Ok(Arc::from_raw(raw as *mut T))
        }
    } else {
        Err(b)
    }
}

/// A wrapper around [`Box<dyn Any>`] that implements [`Serialize`] and [`Deserialize`].

pub type BoxAnySerde = Box<dyn AnySerde>;

#[derive(Clone, Debug)]
pub struct ArcAnySerde(pub Arc<dyn AnySerde>);

impl dyn AnySerde {
    pub fn is<T: Any>(&self) -> bool {
        let t = TypeId::of::<T>();
        let concrete = self.type_id();
        t == concrete
    }
}

impl dyn AnySerde {
    pub fn downcast_ref<T: AnySerde>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const dyn AnySerde as *const T)) }
        } else {
            None
        }
    }
    pub fn downcast_mut<T: AnySerde>(&mut self) -> Option<&T> {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const dyn AnySerde as *const T)) }
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

impl Serialize for ArcAnySerde {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_dyn(&**self)
    }
}

impl<'a> Serialize for &'a dyn AnySerde {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_dyn(*self)
    }
}

impl Clone for BoxAnySerde {
    fn clone(&self) -> Self { self.clone_box() }
}

impl<'de> Deserialize<'de> for BoxAnySerde {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_box()
    }
}

impl<'de> Deserialize<'de> for ArcAnySerde {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_arc()
    }
}

impl ArcAnySerde {
    pub fn new<T: AnySerde>(x: T) -> Self {
        ArcAnySerde(Arc::new(x))
    }
}

impl Deref for ArcAnySerde {
    type Target = dyn AnySerde;
    fn deref(&self) -> &Self::Target { &*self.0 }
}

pub(crate) trait AnyDeserializerDefault<'de>: Deserializer<'de> {
    fn deserialize_box(self) -> Result<BoxAnySerde, Self::Error>;
    fn deserialize_arc(self) -> Result<ArcAnySerde, Self::Error>;
}

/// A trait that extends [`Deserializer`] with the ability to produce [`AnySerde`].
pub trait AnyDeserializer<'de>: Deserializer<'de> {
    fn deserialize_box_impl(self) -> Result<BoxAnySerde, Self::Error>;
    fn deserialize_arc_impl(self) -> Result<ArcAnySerde, Self::Error>;
}

impl<'de, D: Deserializer<'de>> AnyDeserializerDefault<'de> for D {
    default fn deserialize_box(self) -> Result<BoxAnySerde, D::Error> {
        panic!("Missing AnyDeserializerImpl impl for {}", type_name::<D>());
    }
    default fn deserialize_arc(self) -> Result<ArcAnySerde, D::Error> {
        panic!("Missing AnyDeserializerImpl impl for {}", type_name::<D>());
    }
}

impl<'de, D: AnyDeserializer<'de>> AnyDeserializerDefault<'de> for D {
    fn deserialize_box(self) -> Result<BoxAnySerde, D::Error> {
        self.deserialize_box_impl()
    }

    fn deserialize_arc(self) -> Result<ArcAnySerde, Self::Error> {
        self.deserialize_arc_impl()
    }
}

pub(crate) trait AnySerializerDefault: Serializer {
    fn serialize_dyn(self, value: &dyn AnySerde) -> Result<Self::Ok, Self::Error>;
}

/// A trait that extends [`Serializer`] with the ability to consume `&dyn Any`.
pub trait AnySerializer: Serializer {
    fn serialize_dyn_impl(self, value: &dyn AnySerde) -> Result<Self::Ok, Self::Error>;
}

impl<T: Serializer> AnySerializerDefault for T {
    default fn serialize_dyn(self, value: &dyn AnySerde) -> Result<Self::Ok, Self::Error> {
        panic!("AnySerializer not implemented for {:?}", type_name::<T>());
    }
}

impl<T: AnySerializer> AnySerializerDefault for T {
    fn serialize_dyn(self, value: &dyn AnySerde) -> Result<Self::Ok, Self::Error> {
        self.serialize_dyn_impl(value)
    }
}

// Traits for scoping macro contents.
#[doc(hidden)]
pub trait JsonNopTrait { fn nop(); }

#[doc(hidden)]
pub trait BinaryNopTrait { fn nop(); }
