
use serde::{Serializer, Serialize};
use serde::ser::Impossible;
use std::fmt;
use serde_any::ser::{AnySerializer, AnySerialize, AnySerializerImpl, AnySerializeSingleton};
use std::any::TypeId;
pub struct Custom;

impl AnySerializerImpl for Custom {
    fn serialize_dyn_impl(self, value: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
        value.as_serialize_impl(TypeId::of::<dyn AnySerializeCustom>())
            .ok_or(fmt::Error)?
            .downcast_ref::<&'static dyn AnySerializeCustom>()
            .ok_or(fmt::Error)?
            .serialize_custom(self, value)
    }
}

pub trait AnySerializeCustom: 'static + Send + Sync {
    fn serialize_custom(&self, serializer: Custom, value: &dyn AnySerialize) -> Result<(), fmt::Error>;
}

impl<T: Serialize + 'static> AnySerializeCustom for AnySerializeSingleton<T> {
    fn serialize_custom(&self, serializer: Custom, value: &dyn AnySerialize) -> Result<(), fmt::Error> {
        value.as_any().downcast_ref::<T>().serialize(serializer)
    }
}

impl Serializer for Custom {
    type Ok = ();
    type Error = fmt::Error;
    type SerializeSeq = Impossible<(), fmt::Error>;
    type SerializeTuple = Impossible<(), fmt::Error>;
    type SerializeTupleStruct = Impossible<(), fmt::Error>;
    type SerializeTupleVariant = Impossible<(), fmt::Error>;
    type SerializeMap = Impossible<(), fmt::Error>;
    type SerializeStruct = Impossible<(), fmt::Error>;
    type SerializeStructVariant = Impossible<(), fmt::Error>;
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        Ok(())
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        Ok(())
    }
    fn serialize_newtype_variant<T: ?Sized>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<Self::Ok, Self::Error> where T: Serialize {
        Ok(())
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        unimplemented!()
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        unimplemented!()
    }
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        unimplemented!()
    }
    fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unimplemented!()
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!()
    }
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        unimplemented!()
    }
    fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
        unimplemented!()
    }
}

#[test]
fn test_custom(){
}