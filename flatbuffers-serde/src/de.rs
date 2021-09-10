use std::fmt::{Display, Formatter, Debug};
use flatbuffers::{Follow, Table, VOffsetT, WIPOffset, ForwardsUOffset, UOffsetT};
use serde::de::{Visitor, SeqAccess, DeserializeSeed, EnumAccess, VariantAccess, IntoDeserializer};
use std::marker::PhantomData;
use std::mem::size_of;
use crate::{U128, FollowOrNull, VariantT};
use serde::de::value::StrDeserializer;

#[derive(Debug)]
pub enum DeserializeError {
    Custom(String),
    EndOfBuffer,
    BadChar,
    Unsupported,
    MissingEnumValue,
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

pub trait FlatDeserializer<'de>: Debug {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError>;
    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner>;
    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner>;
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError>;
}

#[derive(Copy, Clone, Debug)]
pub struct Deserializer<I> {
    imp: I,
}

#[derive(Debug, Copy, Clone)]
pub struct IdentityDeserializer<'de> {
    buf: &'de [u8],
    loc: usize,
}

#[derive(Debug)]
struct NoneDeserializer;

#[derive(Debug)]
struct RepeatNoneDeserializer;

#[derive(Debug)]
struct FieldDeserializer<'de> {
    buf: &'de [u8],
    loc: usize,
}

#[derive(Debug)]
struct TableDeserializer<'de> {
    table: Table<'de>,
    index: usize,
}

#[derive(Debug)]
struct VectorDeserializer<'de> {
    buf: &'de [u8],
    loc: usize,
    len: usize,
}

impl<'de> IdentityDeserializer<'de> {
    fn follow<T: Follow<'de>>(&self) -> T::Inner {
        T::follow(self.buf, self.loc)
    }
}

impl<'de> FieldDeserializer<'de> {
    fn follow<T: Follow<'de>>(&self) -> T::Inner {
        T::follow(self.buf, self.loc)
    }
}

impl<'de> Follow<'de> for VectorDeserializer<'de> {
    type Inner = VectorDeserializer<'de>;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        let len = UOffsetT::follow(buf, loc);
        let loc = loc + size_of::<UOffsetT>();
        VectorDeserializer {
            buf,
            loc,
            len: len as usize,
        }
    }
}

impl<'de> FlatDeserializer<'de> for IdentityDeserializer<'de> {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        if let Some(deserializer)
        = self.follow::<FollowOrNull<ForwardsUOffset<Deserializer<IdentityDeserializer>>>>() {
            visitor.visit_some(deserializer)
        } else {
            visitor.visit_none()
        }
    }
    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        Some(self.follow::<T>())
    }
    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        Some(self.follow::<T>())
    }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        println!("deserialize_enum({:?})", self);
        self.follow::<TableDeserializer>().deserialize_enum(visitor)
    }
}

impl<'de> FlatDeserializer<'de> for NoneDeserializer {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        visitor.visit_none()
    }

    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        None
    }
    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        None
    }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        struct NoneEnumAccess;
        struct NoneVariantAccess;
        impl<'de> EnumAccess<'de> for NoneEnumAccess {
            type Error = DeserializeError;
            type Variant = NoneVariantAccess;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> where V: DeserializeSeed<'de> {
                Ok((seed.deserialize(0u32.into_deserializer())?, NoneVariantAccess))
            }
        }
        impl<'de> VariantAccess<'de> for NoneVariantAccess {
            type Error = DeserializeError;

            fn unit_variant(self) -> Result<(), Self::Error> {
                Ok(())
            }

            fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error> where T: DeserializeSeed<'de> {
                seed.deserialize(Deserializer { imp: NoneDeserializer })
            }

            fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
                visitor.visit_seq(RepeatNoneDeserializer)
            }

            fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
                visitor.visit_seq(RepeatNoneDeserializer)
            }
        }
        visitor.visit_enum(NoneEnumAccess)
    }
}

impl<'a, 'de> EnumAccess<'de> for &'a mut TableDeserializer<'de> {
    type Error = DeserializeError;
    type Variant = &'a mut TableDeserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> where V: DeserializeSeed<'de> {
        let variant = self.deserialize_fixed::<VariantT>().unwrap_or(0);
        let variant = seed.deserialize(variant.into_deserializer())?;
        Ok((variant, self))
    }
}

impl<'a, 'de> VariantAccess<'de> for &'a mut TableDeserializer<'de> {
    type Error = DeserializeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.index += 1;
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error> where T: DeserializeSeed<'de> {
        let deserializer = self.deserialize_fixed::<FieldDeserializer>();
        let mut deserializer = deserializer.ok_or(DeserializeError::MissingEnumValue)?;
        seed.deserialize(Deserializer { imp: deserializer })
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let deserializer = self.deserialize_fixed::<ForwardsUOffset<TableDeserializer>>();
        let mut deserializer = deserializer.ok_or(DeserializeError::MissingEnumValue)?;
        visitor.visit_seq(&mut deserializer)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.tuple_variant(fields.len(), visitor)
    }
}

impl<'a, 'de> FlatDeserializer<'de> for &'a mut TableDeserializer<'de> {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        if let Some(deserializer) = self.deserialize_fixed::<ForwardsUOffset<IdentityDeserializer>>() {
            visitor.visit_some(Deserializer { imp: deserializer })
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        println!("deserialize_value({:?})", self);
        let result = self.table.get::<T>((self.index * 2 + 4) as u16, None);
        self.index += 1;
        result
    }

    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        println!("deserialize_value({:?})", self);
        let result = self.table.get::<ForwardsUOffset<T>>((self.index * 2 + 4) as u16, None);
        self.index += 1;
        result
    }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        visitor.visit_enum(self)
    }
}


impl<'a, 'de> FlatDeserializer<'de> for FieldDeserializer<'de> {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        self.follow::<ForwardsUOffset<IdentityDeserializer>>().deserialize_option(visitor)
    }

    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        Some(self.follow::<T>())
    }

    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        Some(self.follow::<ForwardsUOffset<T>>())
    }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        self.follow::<ForwardsUOffset<IdentityDeserializer>>().deserialize_enum(visitor)
    }
}

impl<'a, 'de> FlatDeserializer<'de> for &'a mut VectorDeserializer<'de> {
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        println!("deserialize_option({:?})", self);
        let deserializer = self.deserialize_fixed::<FollowOrNull<ForwardsUOffset<IdentityDeserializer>>>().unwrap();
        if let Some(deserializer) = deserializer {
            visitor.visit_some(Deserializer { imp: deserializer })
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_fixed<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        let result = T::follow(self.buf, self.loc);
        self.loc += size_of::<T>();
        self.len -= 1;
        Some(result)
    }
    fn deserialize_variable<T: Follow<'de> + 'de>(self) -> Option<T::Inner> {
        self.deserialize_fixed::<ForwardsUOffset<T>>()
    }
    fn deserialize_enum<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeserializeError> {
        self.deserialize_fixed::<ForwardsUOffset<IdentityDeserializer>>().unwrap().deserialize_enum(visitor)
    }
}

impl<'de> Follow<'de> for IdentityDeserializer<'de> {
    type Inner = Self;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        IdentityDeserializer {
            buf,
            loc,
        }
    }
}

impl<'de> Follow<'de> for TableDeserializer<'de> {
    type Inner = Self;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        TableDeserializer {
            table: Table::follow(buf, loc),
            index: 0,
        }
    }
}

impl<'de> Follow<'de> for FieldDeserializer<'de> {
    type Inner = Self;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        FieldDeserializer {
            buf,
            loc,
        }
    }
}

impl<'de, I: Follow<'de>> Follow<'de> for Deserializer<I> {
    type Inner = Deserializer<I::Inner>;
    fn follow(buf: &'de [u8], loc: usize) -> Self::Inner {
        Deserializer { imp: I::follow(buf, loc) }
    }
}

impl<'de> SeqAccess<'de> for TableDeserializer<'de> {
    type Error = DeserializeError;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
        println!("next_element_seed {:?}", self.index);
        Ok(Some(seed.deserialize(Deserializer { imp: self })?))
    }
}

impl<'de> SeqAccess<'de> for VectorDeserializer<'de> {
    type Error = DeserializeError;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
        if self.len == 0 {
            Ok(None)
        } else {
            Ok(Some(seed.deserialize(Deserializer { imp: self })?))
        }
    }
}

impl<'de> SeqAccess<'de> for RepeatNoneDeserializer {
    type Error = DeserializeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
        Ok(Some(seed.deserialize(Deserializer { imp: NoneDeserializer })?))
    }
}

impl<'de> SeqAccess<'de> for NoneDeserializer {
    type Error = DeserializeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
        Ok(None)
    }
}

impl<'de, I> serde::Deserializer<'de> for Deserializer<I> where I: FlatDeserializer<'de> {
    type Error = DeserializeError;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        Err(DeserializeError::Unsupported)
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_bool(self.imp.deserialize_fixed::<bool>().unwrap_or_default())
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_i8(self.imp.deserialize_fixed::<i8>().unwrap_or_default())
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
        visitor.visit_u8(self.imp.deserialize_fixed::<u8>().unwrap_or_default())
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u16(self.imp.deserialize_fixed::<u16>().unwrap_or_default())
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u32(todo!())
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u64(todo!())
    }
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_u128(self.imp.deserialize_fixed::<U128>().unwrap_or_default())
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
        self.imp.deserialize_option(visitor)
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let mut deserializer = self.imp.deserialize_variable::<TableDeserializer>();
        if let Some(mut deserializer) = deserializer {
            let deserializer = Deserializer { imp: &mut deserializer };
            visitor.visit_newtype_struct(deserializer)
        } else {
            visitor.visit_newtype_struct(Deserializer { imp: NoneDeserializer })
        }
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        let mut deserializer = self.imp.deserialize_variable::<VectorDeserializer>();
        if let Some(mut deserializer) = deserializer {
            visitor.visit_seq(deserializer)
        } else {
            visitor.visit_seq(NoneDeserializer)
        }
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        println!("deserialize_tuple({:?})", self.imp);
        let mut deserializer = self.imp.deserialize_variable::<TableDeserializer>();
        if let Some(mut deserializer) = deserializer {
            visitor.visit_seq(deserializer)
        } else {
            visitor.visit_seq(RepeatNoneDeserializer)
        }
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
        println!("deserialize_enum({:?})", self);
        self.imp.deserialize_enum(visitor)
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        todo!()
    }
}