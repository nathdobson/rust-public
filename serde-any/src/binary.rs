use serde::{Serializer, Serialize, Deserializer, Deserialize};
use std::fmt::{Display, Debug, Formatter};
use serde::ser::{SerializeSeq, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, SerializeMap, SerializeStruct, SerializeStructVariant};
use std::ops::Range;
use std::io::{Cursor, Read};
use serde::de::{Visitor, SeqAccess, MapAccess, EnumAccess, IntoDeserializer, VariantAccess};
use std::io;
use std::string::FromUtf8Error;
use serde::de::DeserializeSeed;
use crate::binary::Error::Unsupported;
use crate::tag::TypeTag;

pub struct BinarySerializer<'a> {
    vec: &'a mut Vec<u8>,
}

pub struct BinaryDeserializer<'de> {
    cursor: Cursor<&'de [u8]>,
}

pub struct BinaryCountSerializer<'a> {
    serializer: BinarySerializer<'a>,
    count_index: usize,
    count: usize,
}

#[derive(Debug)]
pub enum Error {
    Custom(String),
    Io(io::Error),
    FromUtf8(FromUtf8Error),
    BadChar,
    Unsupported,
    MissingImpl,
    BadImpl,
    BadType,
}

type Result<T> = std::result::Result<T, Error>;

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Custom(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            Error::BadChar => write!(f, "Char not in unicode range."),
            Error::FromUtf8(e) => write!(f, "{}", e),
            Error::Unsupported => write!(f, "Unsupported operation"),
            Error::MissingImpl => write!(f, "Missing AnySerializeBinary"),
            Error::BadImpl => write!(f, "Bad AnySerializeBinary"),
            Error::BadType => write!(f, "Bad AnySerialize"),
        }
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self where T: Display {
        Error::Custom(format!("{}", msg))
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self where T: Display {
        Error::Custom(format!("{}", msg))
    }
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
        Ok(self)
    }
    fn serialize_map(mut self, len: Option<usize>) -> Result<BinaryCountSerializer<'a>> {
        self.serialize_counted()
    }
    fn serialize_struct(mut self, name: &'static str, len: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_struct_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self> {
        Ok(self)
    }
}

impl From<io::Error> for Error {
    fn from(ioe: io::Error) -> Self { Error::Io(ioe) }
}

impl From<FromUtf8Error> for Error {
    fn from(fue: FromUtf8Error) -> Self { Error::FromUtf8(fue) }
}

impl<'de> BinaryDeserializer<'de> {
    pub fn new(slice: &'de [u8]) -> Self {
        BinaryDeserializer { cursor: Cursor::new(slice) }
    }
    pub fn read_fixed<const C: usize>(&mut self) -> Result<[u8; C]> {
        let mut buf = [0u8; C];
        self.cursor.read_exact(&mut buf)?;
        Ok(buf)
    }
}

impl<'a, 'de> EnumAccess<'de> for &'a mut BinaryDeserializer<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    {
        let index: u32 = u32::from_le_bytes(self.read_fixed()?);
        let val = DeserializeSeed::deserialize(seed,
                                               IntoDeserializer::<Error>::into_deserializer(index))?;
        Ok((val, self))
    }
}

impl<'a, 'de> VariantAccess<'de> for &'a mut BinaryDeserializer<'de> {
    type Error = Error;
    fn unit_variant(self) -> Result<()> {
        Ok(())
    }
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value> where T: DeserializeSeed<'de> {
        DeserializeSeed::deserialize(seed, self)
    }
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        self.deserialize_tuple(len, visitor)
    }
    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        self.deserialize_tuple(fields.len(), visitor)
    }
}

impl<'a, 'de> Deserializer<'de> for &'a mut BinaryDeserializer<'de> {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        Err(Error::Unsupported)
    }
    fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_bool(u8::from_le_bytes(self.read_fixed()?) != 0)
    }
    fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_i8(i8::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_i16(i16::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_i32(i32::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_i64(i64::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_i128<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_i128(i128::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_u8(u8::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_u16(u16::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_u32(u32::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_u64(u64::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_u128<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_u128(u128::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_f32(f32::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_f64(f64::from_le_bytes(self.read_fixed()?))
    }
    fn deserialize_char<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_char(
            char::from_u32(u32::from_le_bytes(self.read_fixed()?))
                .ok_or(Error::BadChar)?)
    }
    fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        self.deserialize_string(visitor)
    }
    fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        let length = u64::from_le_bytes(self.read_fixed()?);
        let mut vec = vec![0; length as usize];
        self.cursor.read_exact(&mut vec)?;
        visitor.visit_string(String::from_utf8(vec)?)
    }
    fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        self.deserialize_byte_buf(visitor)
    }
    fn deserialize_byte_buf<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        let length = u64::from_le_bytes(self.read_fixed()?);
        let mut vec = vec![0; length as usize];
        self.cursor.read_exact(&mut vec)?;
        visitor.visit_byte_buf(vec)
    }
    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        if u8::from_le_bytes(self.read_fixed()?) != 0 {
            visitor.visit_some(self)
        } else {
            visitor.visit_none()
        }
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        let length = u64::from_le_bytes(self.read_fixed()?);
        self.deserialize_tuple(length as usize, visitor)
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        struct Access<'a, 'de> {
            de: &'a mut BinaryDeserializer<'de>,
            len: usize,
        }
        impl<'a, 'de> SeqAccess<'de> for Access<'a, 'de> {
            type Error = Error;
            fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
                if self.len > 0 {
                    self.len -= 1;
                    let value = seed.deserialize(&mut *self.de)?;
                    Ok(Some(value))
                } else {
                    Ok(None)
                }
            }
            fn size_hint(&self) -> Option<usize> { Some(self.len) }
        }
        visitor.visit_seq(Access { de: self, len })
    }
    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        self.deserialize_tuple(len, visitor)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        let len = u64::from_le_bytes(self.read_fixed()?) as usize;
        struct Access<'a, 'de> {
            de: &'a mut BinaryDeserializer<'de>,
            len: usize,
        }
        impl<'a, 'de> MapAccess<'de> for Access<'a, 'de> {
            type Error = Error;
            fn size_hint(&self) -> Option<usize> { Some(self.len) }
            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>> where K: DeserializeSeed<'de> {
                if self.len > 0 {
                    self.len -= 1;
                    let key = seed.deserialize(&mut *self.de)?;
                    Ok(Some(key))
                } else {
                    Ok(None)
                }
            }
            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value> where V: DeserializeSeed<'de> {
                seed.deserialize(&mut *self.de)
            }
        }
        visitor.visit_map(Access { de: self, len })
    }
    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        self.deserialize_tuple(fields.len(), visitor)
    }
    fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        visitor.visit_enum(self)
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        Err(Unsupported)
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value> where V: Visitor<'de> {
        Err(Unsupported)
    }
}

pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut vec = vec![];
    value.serialize(BinarySerializer::new(&mut vec))?;
    Ok(vec)
}

pub fn deserialize<'de, T: Deserialize<'de>>(slice: &'de [u8]) -> Result<T> {
    T::deserialize(&mut BinaryDeserializer::new(slice))
}