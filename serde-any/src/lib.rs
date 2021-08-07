#![feature(specialization)]
#![allow(incomplete_features, unused_variables, dead_code, unused_imports)]
#![deny(unused_must_use)]


// mod test;

use serde::{Serializer, Serialize};
use std::any::{Any, type_name};
use std::ops::{DerefMut, Deref};
use bincode::{Error, Options};

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

impl<S: Serializer> AnySerializerOuter for S {
    default fn serialize_any_outer(self, any: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
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

impl Serialize for AnySerde {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_any_outer(&*self.inner)
    }
}

fn serialize_bincode<T: Serialize>(input: &T) -> Result<Vec<u8>, bincode::Error> {
    let mut vec = vec![];
    let mut serializer =
        bincode::Serializer::new(
            &mut vec,
            bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes(),
        );
    input.serialize(&mut serializer)?;
    Ok(vec)
}

fn serialize_json<T: Serialize>(input: &T) -> Result<String, serde_json::Error> {
    let mut vec = vec![];
    let mut serializer = serde_json::Serializer::new(&mut vec);
    input.serialize(&mut serializer)?;
    Ok(String::from_utf8(vec).unwrap())
}

#[test]
fn test_serialize_bincode() {
    assert_eq!(vec![10, 0, 0, 0], bincode::serialize(&10i32).unwrap());
    assert_eq!(vec![10, 0, 0, 0], serialize_bincode(&AnySerde::new(10i32)).unwrap());
}

#[test]
fn test_serialize_json() {
    assert_eq!("10", serialize_json(&AnySerde::new(10)).unwrap());
}


// struct AnySerializer<S> {
//     inner: S,
// }
//
// impl<S: Serializer> Serializer for AnySerializer<S> {
//     type Ok = S::Ok;
//     type Error = S::Error;
//     type SerializeSeq = S::SerializeSeq;
//     type SerializeTuple = S::SerializeTuple;
//     type SerializeTupleStruct = S::SerializeTupleStruct;
//     type SerializeTupleVariant = S::SerializeTupleVariant;
//     type SerializeMap = S::SerializeMap;
//     type SerializeStruct = S::SerializeStruct;
//     type SerializeStructVariant = S::SerializeStructVariant;
//
//     fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_bool(v)
//     }
//
//     fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_i8(v)
//     }
//
//     fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_i16(v)
//     }
//
//     fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_i32(v)
//     }
//
//     fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_i64(v)
//     }
//
//     fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_u8(v)
//     }
//
//     fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_u16(v)
//     }
//
//     fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_u32(v)
//     }
//
//     fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_u64(v)
//     }
//
//     fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_f32(v)
//     }
//
//     fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_f64(v)
//     }
//
//     fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_char(v)
//     }
//
//     fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_str(v)
//     }
//
//     fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_bytes(v)
//     }
//
//     fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_none()
//     }
//
//     fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
//         self.inner.serialize_some(value)
//     }
//
//     fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_unit()
//     }
//
//     fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_unit_struct(name)
//     }
//
//     fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<Self::Ok, Self::Error> {
//         self.inner.serialize_unit_variant(name, variant_index, variant)
//     }
//
//     fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
//         self.inner.serialize_newtype_struct(name, value)
//     }
//
//     fn serialize_newtype_variant<T: ?Sized>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
//         self.inner.serialize_newtype_variant(name, variant_index, variant, value)
//     }
//
//     fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
//         self.inner.serialize_seq(len)
//     }
//
//     fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
//         self.inner.serialize_tuple(len)
//     }
//
//     fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
//         self.inner.serialize_tuple_struct(name, len)
//     }
//
//     fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
//         self.inner.serialize_tuple_variant(name, variant_index, variant, len)
//     }
//
//     fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
//         self.inner.serialize_map(len)
//     }
//
//     fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
//         self.inner.serialize_struct(name, len)
//     }
//
//     fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
//         self.inner.serialize_struct_variant(name, variant_index, variant, len)
//     }
// }

// trait SerializerExt: Serializer {
//     fn serialize_any(self, any: &dyn Any) -> Result<Self::Ok, Self::Error>;
// }
//
// default impl<S: Serializer> SerializerExt for S {
//     fn serialize_any(self, any: &dyn Any) -> Result<Self::Ok, Self::Error> {
//         panic!("Must use AnySerializer ")
//     }
// }
//
// impl<S: Serializer> SerializerExt for AnySerializer<S> {
//     fn serialize_any(self, any: &dyn Any) -> Result<Self::Ok, Self::Error> {
//         todo!()
//     }
// }

//
// trait AnySerializer {
//     fn foo() {}
// }
//
// impl<T: Serializer> AnySerializer for T {
//     fn foo() {}
// }
//
// default impl<W: Write, O: bincode::Options> AnySerializer for bincode::Serializer<W, O> {
//     fn foo() {}
// }
//
// trait SerdeAnyInner {
//     fn serialize2(&self, serializer: &mut dyn AnySerializer);
// }
//
// impl Serialize for SerdeAny {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
//         let mut serializer: Option<S> = Some(serializer);
//         let mut result: Option<Result<S::Ok, S::Error>> = None;
//         self.inner.serialize2(&mut serializer);
//         result.unwrap()
//     }
// }