use std::fmt::{Display, Formatter};
use flatbuffers::{Follow, Table, VOffsetT, WIPOffset, ForwardsUOffset, UOffsetT};
use serde::de::{Visitor, SeqAccess, DeserializeSeed};
use std::marker::PhantomData;

#[derive(Debug)]
pub enum DeserializeError {
    Custom(String),
    EndOfBuffer,
    BadChar,
    Unsupported,
}

impl std::error::Error for DeserializeError {}

impl Display for DeserializeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl serde::de::Error for DeserializeError {
    fn custom<T>(msg: T) -> Self where T: Display {
        DeserializeError::Custom(msg.to_string())
    }
}

pub trait FlatDeserializer<'de> {
    fn flat_deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError>;
    fn flat_deserialize_primitive<T: Follow<'de>>(self) -> T::Inner;
}

#[derive(Copy, Clone, Debug)]
pub struct Deserializer<'de, M> {
    buf: &'de [u8],
    loc: usize,
    phantom: PhantomData<M>,
}

#[derive(Debug, Copy, Clone)]
pub struct IsIdentity;

#[derive(Debug, Copy, Clone)]
pub struct IsOption;

#[derive(Debug, Copy, Clone)]
pub struct IsNone;

type IdentityDeserializer<'de> = Deserializer<'de, IsIdentity>;
type OptionDeserializer<'de> = Deserializer<'de, IsOption>;
struct NoneDeserializer;

impl<'de, M> Deserializer<'de, M> {
    fn follow<T: Follow<'de>>(&self) -> T::Inner {
        T::follow(self.buf, self.loc)
    }
    fn cast<M2>(self) -> Deserializer<'de, M2> {
        Deserializer {
            buf: self.buf,
            loc: self.loc,
            phantom: PhantomData,
        }
    }
}

impl<'de> FlatDeserializer<'de> for IdentityDeserializer<'de> {
    fn flat_deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        if self.follow::<UOffsetT>() == 0 {
            visitor.visit_none()
        } else {
            visitor.visit_some(self.cast::<IsOption>())
        }
    }
    fn flat_deserialize_primitive<T: Follow<'de>>(self) -> T::Inner {
        self.follow::<T>()
    }
}

impl<'de> FlatDeserializer<'de> for OptionDeserializer<'de> {
    fn flat_deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        self.follow::<ForwardsUOffset<IdentityDeserializer>>().flat_deserialize_option(visitor)
    }
    fn flat_deserialize_primitive<T: Follow<'de>>(self) -> T::Inner {
        self.follow::<ForwardsUOffset<T>>()
    }
}

impl<'de, M> Follow<'de> for Deserializer<'de, M> {
    type Inner = Self;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        Deserializer {
            buf,
            loc,
            phantom: PhantomData,
        }
    }
}

impl<'de, T> serde::Deserializer<'de> for Deserializer<'de, T> where Self: FlatDeserializer<'de> {
    type Error = DeserializeError;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_bool(self.flat_deserialize_primitive::<bool>())
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_i8(self.flat_deserialize_primitive::<i8>())
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_i16(todo!())
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_i32(todo!())
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_i64(todo!())
    }
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_i128(todo!())
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u8(self.flat_deserialize_primitive::<u8>())
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u16(self.flat_deserialize_primitive::<u16>())
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u32(todo!())
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u64(todo!())
    }
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u128(todo!())
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_f32(todo!())
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_f64(todo!())
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_char(std::char::from_u32(todo!()).ok_or(DeserializeError::BadChar)?)
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.flat_deserialize_option(visitor)
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let table = Table::follow(self.buf, self.loc);
        let buf = table.buf;
        let off = table.vtable().get(4);
        let value = if off == 0 {
            visitor.visit_newtype_struct(NoneDeserializer)?
        } else {
            visitor.visit_newtype_struct(IdentityDeserializer {
                buf,
                loc: table.loc + off as usize,
                phantom: PhantomData,
            })?
        };
        Ok(value)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        struct Seq<'de> {
            index: VOffsetT,
            len: VOffsetT,
            table: Table<'de>,
        }
        impl<'de> SeqAccess<'de> for Seq<'de> {
            type Error = DeserializeError;
            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
                if self.index == self.len {
                    return Ok(None);
                }
                let buf = self.table.buf;
                let off = self.table.vtable().get(self.index * 2 + 4);
                let value = if off == 0 {
                    seed.deserialize(NoneDeserializer)?
                } else {
                    seed.deserialize(IdentityDeserializer {
                        buf,
                        loc: self.table.loc + off as usize,
                        phantom: PhantomData,
                    })?
                };
                self.index += 1;
                Ok(Some(value))
            }
        }
        visitor.visit_seq(Seq {
            index: 0,
            len: len as u16,
            table: Table::follow(self.buf, self.loc),
        })
    }
    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
}

impl<'de> serde::Deserializer<'de> for NoneDeserializer {
    type Error = DeserializeError;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u8(0)
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_none()
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
}