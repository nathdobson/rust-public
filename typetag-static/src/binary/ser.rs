use serde::{Serialize, Serializer};
use serde::ser::{SerializeSeq, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, SerializeMap, SerializeStruct, SerializeStructVariant};
use crate::binary::Error;
use crate::binary::Result;

pub struct BinarySerializer<'a> {
    vec: &'a mut Vec<u8>,
}

pub struct BinaryCountSerializer<'a> {
    serializer: BinarySerializer<'a>,
    count_index: usize,
    count: usize,
}


impl<'a> BinarySerializer<'a> {
    pub fn new(vec: &'a mut Vec<u8>) -> Self {
        BinarySerializer { vec }
    }
    pub fn reborrow<'b>(&'b mut self) -> BinarySerializer<'b> {
        BinarySerializer { vec: &mut *self.vec }
    }
    pub fn serialize_raw(&mut self, bytes: &[u8]) -> Result<()> {
        self.vec.extend_from_slice(bytes);
        Ok(())
    }
    fn serialize_counted(mut self) -> Result<BinaryCountSerializer<'a>> {
        let count_index = self.vec.len();
        self.vec.resize(self.vec.len() + 8, 0);
        Ok(BinaryCountSerializer {
            serializer: self,
            count_index,
            count: 0,
        })
    }
    pub fn serialize_with_length<T: Serialize>(mut self, element: &T) -> Result<()> {
        let mut counter = self.serialize_counted()?;
        let start = counter.serializer.vec.len();
        element.serialize(counter.serializer.reborrow())?;
        counter.count = counter.serializer.vec.len() - start;
        counter.end_count()?;
        Ok(())
    }
}

impl<'a> BinaryCountSerializer<'a> {
    fn end_count(self) -> Result<()> {
        self.serializer.vec[self.count_index..self.count_index + 8]
            .copy_from_slice(&self.count.to_le_bytes());
        Ok(())
    }
}

impl<'a> SerializeSeq for BinaryCountSerializer<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        value.serialize(self.serializer.reborrow())?;
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<()> {
        self.end_count()
    }
}

impl<'a> SerializeTuple for BinarySerializer<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        value.serialize(self.reborrow())
    }
    fn end(self) -> Result<()> { Ok(()) }
}

impl<'a> SerializeTupleStruct for BinarySerializer<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        value.serialize(self.reborrow())
    }
    fn end(self) -> Result<()> { Ok(()) }
}

impl<'a> SerializeTupleVariant for BinarySerializer<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        value.serialize(self.reborrow())
    }
    fn end(self) -> Result<()> { Ok(()) }
}

impl<'a> SerializeMap for BinaryCountSerializer<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()> where T: Serialize {
        key.serialize(self.serializer.reborrow())?;
        self.count += 1;
        Ok(())
    }
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        value.serialize(self.serializer.reborrow())
    }
    fn end(self) -> Result<()> { self.end_count() }
}

impl<'a> SerializeStruct for BinarySerializer<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> where T: Serialize {
        value.serialize(self.reborrow())
    }
    fn end(self) -> Result<()> { Ok(()) }
}

impl<'a> SerializeStructVariant for BinarySerializer<'a> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> where T: Serialize {
        value.serialize(self.reborrow())
    }
    fn end(self) -> Result<()> { Ok(()) }
}

impl<'a> Serializer for BinarySerializer<'a> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = BinaryCountSerializer<'a>;
    type SerializeTuple = BinarySerializer<'a>;
    type SerializeTupleStruct = BinarySerializer<'a>;
    type SerializeTupleVariant = BinarySerializer<'a>;
    type SerializeMap = BinaryCountSerializer<'a>;
    type SerializeStruct = BinarySerializer<'a>;
    type SerializeStructVariant = BinarySerializer<'a>;
    fn serialize_bool(mut self, v: bool) -> Result<()> {
        self.serialize_raw(&[v as u8])
    }
    fn serialize_i8(mut self, v: i8) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_i16(mut self, v: i16) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_i32(mut self, v: i32) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_i64(mut self, v: i64) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_i128(mut self, v: i128) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_u8(mut self, v: u8) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_u16(mut self, v: u16) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_u32(mut self, v: u32) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_u64(mut self, v: u64) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_u128(mut self, v: u128) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_f32(mut self, v: f32) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_f64(mut self, v: f64) -> Result<()> {
        self.serialize_raw(&v.to_le_bytes())
    }
    fn serialize_char(mut self, v: char) -> Result<()> {
        self.serialize_raw(&(v as u32).to_le_bytes())
    }
    fn serialize_str(mut self, v: &str) -> Result<()> {
        self.serialize_bytes(v.as_bytes())
    }
    fn serialize_bytes(mut self, v: &[u8]) -> Result<()> {
        self.reborrow().serialize_u64(v.len() as u64)?;
        self.serialize_raw(v)?;
        Ok(())
    }
    fn serialize_none(mut self) -> Result<()> {
        self.serialize_bool(false)
    }
    fn serialize_some<T: ?Sized>(mut self, value: &T) -> Result<()> where T: Serialize {
        self.reborrow().serialize_bool(true)?;
        value.serialize(self)?;
        Ok(())
    }
    fn serialize_unit(mut self) -> Result<()> {
        Ok(())
    }
    fn serialize_unit_struct(mut self, name: &'static str) -> Result<()> {
        Ok(())
    }
    fn serialize_unit_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<()> {
        self.serialize_u32(variant_index)
    }
    fn serialize_newtype_struct<T: ?Sized>(mut self, name: &'static str, value: &T) -> Result<()> where T: Serialize {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized>(mut self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<()> where T: Serialize {
        self.reborrow().serialize_u32(variant_index)?;
        value.serialize(self)?;
        Ok(())
    }
    fn serialize_seq(mut self, len: Option<usize>) -> Result<BinaryCountSerializer<'a>> {
        self.serialize_counted()
    }
    fn serialize_tuple(mut self, len: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_struct(mut self, name: &'static str, len: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self> {
        self.reborrow().serialize_u32(variant_index)?;
        Ok(self)
    }
    fn serialize_map(mut self, len: Option<usize>) -> Result<BinaryCountSerializer<'a>> {
        self.serialize_counted()
    }
    fn serialize_struct(mut self, name: &'static str, len: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_struct_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self> {
        self.reborrow().serialize_u32(variant_index)?;
        Ok(self)
    }
}