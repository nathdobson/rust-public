use std::any::{type_name, Any, TypeId};
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::de::{
    DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor,
};
use serde::{Deserialize, Deserializer, Serialize};

use crate::error::Error;
use crate::{Schema, SchemaId, SchemaMap, SchemaType, VariantSchema};

#[derive(Debug)]
struct SchemaMapBuilder {
    map: HashMap<SchemaId, SchemaType>,
    versions: HashMap<String, usize>,
    ids: HashMap<TypeId, SchemaId>,
}

struct SchemaBuilder<'b> {
    map: &'b mut SchemaMapBuilder,
    id: Option<SchemaId>,
}

struct SeqBuilder<'b> {
    map: &'b mut SchemaMapBuilder,
    seq: Vec<SchemaId>,
    len: usize,
}

struct VariantBuilder<'b> {
    map: &'b mut SchemaMapBuilder,
    enu: Option<VariantSchema>,
    index: u32,
}

trait CustomSchema {
    fn custom_schema(map: &mut SchemaMapBuilder) -> Result<Schema, Error>;
}

impl<T> CustomSchema for T {
    default fn custom_schema(map: &mut SchemaMapBuilder) -> Result<Schema, Error> {
        unimplemented!()
    }
}

impl<T> CustomSchema for Vec<T> {
    fn custom_schema(map: &mut SchemaMapBuilder) -> Result<Schema, Error> {
        Ok(Schema::Vec(map.add_impl::<T>()?.0))
    }
}

impl<K, V> CustomSchema for HashMap<K, V> {
    fn custom_schema(map: &mut SchemaMapBuilder) -> Result<Schema, Error> {
        Ok(Schema::Map(map.add_impl::<K>()?.0, map.add_impl::<V>()?.0))
    }
}

impl<K, V> CustomSchema for BTreeMap<K, V> {
    fn custom_schema(map: &mut SchemaMapBuilder) -> Result<Schema, Error> {
        Ok(Schema::Map(map.add_impl::<K>()?.0, map.add_impl::<V>()?.0))
    }
}

trait MaybeDeserialize<'de>: Sized {
    fn maybe_deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error>;
    fn maybe_type_id() -> TypeId;
}

impl<'de, T> MaybeDeserialize<'de> for T {
    default fn maybe_deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        unimplemented!()
    }
    default fn maybe_type_id() -> TypeId { unimplemented!() }
}

impl<'de, T: Deserialize<'de> + Any> MaybeDeserialize<'de> for T {
    fn maybe_deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::deserialize(d)
    }
    fn maybe_type_id() -> TypeId { TypeId::of::<T>() }
}

impl SchemaMapBuilder {
    pub fn new() -> Self {
        SchemaMapBuilder {
            map: HashMap::new(),
            versions: HashMap::new(),
            ids: HashMap::new(),
        }
    }
    fn add<'de, T: Deserialize<'de>>(&mut self) -> Result<SchemaId, Error> {
        Ok(self.add_impl::<T>()?.0)
    }
    fn get<'de, T: Deserialize<'de> + Any>(&self) -> SchemaId {
        *self.ids.get(&TypeId::of::<T>()).unwrap()
    }
    fn add_impl<T>(&mut self) -> Result<(SchemaId, T), Error> {
        let type_id = T::maybe_type_id();
        let mut builder = self.schema_builder();
        let result = T::maybe_deserialize(&mut builder)?;
        let id = builder.id.unwrap();
        assert_eq!(*self.ids.get(&type_id).unwrap(), id);
        Ok((id, result))
    }
    fn schema_builder<'c>(&'c mut self) -> SchemaBuilder<'c> {
        SchemaBuilder {
            map: self,
            id: None,
        }
    }
    fn seq_builder<'c>(&'c mut self, len: usize) -> SeqBuilder<'c> {
        SeqBuilder {
            map: self,
            seq: vec![],
            len,
        }
    }
    fn variant_builder<'c>(&'c mut self, index: u32) -> VariantBuilder<'c> {
        VariantBuilder {
            map: self,
            enu: None,
            index,
        }
    }
    pub fn build(self) -> SchemaMap { SchemaMap { map: self.map } }
}

impl<'b> SchemaBuilder<'b> {
    fn set<'de, V: Visitor<'de>>(&mut self, schema: Schema) {
        if self.start::<V>() {
            self.finish::<V>(schema)
        }
    }
    fn start<'de, V: Visitor<'de>>(&mut self) -> bool {
        let type_id = V::Value::maybe_type_id();
        let next = self.map.ids.len() as u64;
        match self.map.ids.entry(type_id) {
            Entry::Occupied(o) => {
                self.id = Some(*o.get());
                return false;
            }
            Entry::Vacant(v) => {
                self.id = Some(SchemaId(next));
                v.insert(SchemaId(next));
                return true;
            }
        }
    }
    fn finish<'de, V: Visitor<'de>>(&mut self, schema: Schema) {
        let name = type_name::<V::Value>();
        let version_ref = self.map.versions.entry(name.to_string()).or_default();
        let version = *version_ref;
        *version_ref += 1;
        assert!(self
            .map
            .map
            .insert(
                self.id.unwrap(),
                SchemaType {
                    name: name.to_string(),
                    version,
                    schema,
                }
            )
            .is_none())
    }
}

impl<'a, 'b, 'de> SeqAccess<'de> for &'a mut SeqBuilder<'b> {
    type Error = Error;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.len == 0 {
            Ok(None)
        } else {
            self.len -= 1;
            let (id, result) = self.map.add_impl::<T::Value>()?;
            self.seq.push(id);
            Ok(Some(result))
        }
    }
}

impl<'de, 'a, 'b> Deserializer<'de> for &'a mut SchemaBuilder<'b> {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Bool);
        visitor.visit_bool(false)
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Signed(8));
        visitor.visit_i8(0)
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Signed(16));
        visitor.visit_i16(0)
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Signed(32));
        visitor.visit_i32(0)
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Signed(64));
        visitor.visit_i64(0)
    }
    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Signed(128));
        visitor.visit_i128(0)
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Unsigned(8));
        visitor.visit_u8(0)
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Unsigned(16));
        visitor.visit_u16(0)
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Unsigned(32));
        visitor.visit_u32(0)
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Unsigned(64));
        visitor.visit_u64(0)
    }
    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Unsigned(128));
        visitor.visit_u128(0)
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Float(32));
        visitor.visit_f32(0.0)
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Float(64));
        visitor.visit_f64(0.0)
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Char);
        visitor.visit_char('a')
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::String);
        visitor.visit_str("")
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::String);
        visitor.visit_str("")
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Bytes);
        visitor.visit_bytes(&[])
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Bytes);
        visitor.visit_bytes(&[])
    }
    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.start::<V>() {
            let mut inner = self.map.schema_builder();
            let result = visitor.visit_some(&mut inner)?;
            let inner = inner.id.unwrap();
            self.finish::<V>(Schema::Option(inner));
            Ok(result)
        } else {
            visitor.visit_none()
        }
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::Unit);
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.set::<V>(Schema::UnitStruct {
            name: name.to_string(),
        });
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.start::<V>();
        let mut inner = self.map.schema_builder();
        let result = visitor.visit_newtype_struct(&mut inner)?;
        let inner = inner.id.unwrap();
        self.finish::<V>(Schema::NewtypeStruct {
            name: name.to_string(),
            value: inner,
        });
        Ok(result)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.start::<V>();
        let schema = <V::Value>::custom_schema(self.map)?;
        self.finish::<V>(schema);
        struct Empty;
        impl<'de> SeqAccess<'de> for Empty {
            type Error = Error;
            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                Ok(None)
            }
        }
        visitor.visit_seq(Empty)
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.start::<V>();
        let mut inner = self.map.seq_builder(len);
        let result = visitor.visit_seq(&mut inner)?;
        let inner = inner.seq;
        self.finish::<V>(Schema::Tuple(inner));
        Ok(result)
    }
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.start::<V>();
        let mut inner = self.map.seq_builder(len);
        let result = visitor.visit_seq(&mut inner)?;
        let inner = inner.seq;
        self.finish::<V>(Schema::TupleStruct {
            name: name.to_string(),
            fields: inner,
        });
        Ok(result)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.start::<V>();
        let schema = <V::Value>::custom_schema(self.map)?;
        self.finish::<V>(schema);
        struct Empty;
        impl<'de> MapAccess<'de> for Empty {
            type Error = Error;
            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
            where
                K: DeserializeSeed<'de>,
            {
                Ok(None)
            }
            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
            where
                V: DeserializeSeed<'de>,
            {
                unimplemented!()
            }
        }
        visitor.visit_map(Empty)
    }
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let building = self.start::<V>();
        let mut inner = self.map.seq_builder(fields.len());
        let result = visitor.visit_seq(&mut inner)?;
        if building {
            let inner = inner.seq;
            self.finish::<V>(Schema::Struct {
                name: name.to_string(),
                fields: fields
                    .iter()
                    .map(|x| x.to_string())
                    .zip(inner.into_iter())
                    .collect(),
            });
        }
        Ok(result)
    }
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if variants.is_empty() {
            todo!("empty enum");
        }
        if self.start::<V>() {
            let mut result = None;
            let mut enums = Vec::with_capacity(variants.len());
            for (i, variant) in variants.iter().enumerate() {
                let mut builder = self.map.variant_builder(i as u32);
                result = Some(V::Value::maybe_deserialize(&mut builder)?);
                enums.push(builder.enu.unwrap());
            }
            self.finish::<V>(Schema::Enum {
                enums: variants
                    .iter()
                    .map(|x| x.to_string())
                    .zip(enums.into_iter())
                    .collect(),
            });
            Ok(result.unwrap())
        } else {
            let mut builder = self.map.variant_builder(0);
            V::Value::maybe_deserialize(&mut builder)
        }
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
}

impl<'de, 'a, 'b> Deserializer<'de> for &'a mut VariantBuilder<'b> {
    type Error = Error;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self)
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }
}

impl<'a, 'b, 'de> EnumAccess<'de> for &'a mut VariantBuilder<'b> {
    type Error = Error;
    type Variant = &'a mut VariantBuilder<'b>;
    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let result = seed.deserialize(self.index.into_deserializer())?;
        Ok((result, self))
    }
}

impl<'a, 'b, 'de> VariantAccess<'de> for &'a mut VariantBuilder<'b> {
    type Error = Error;
    fn unit_variant(self) -> Result<(), Self::Error> {
        self.enu = Some(VariantSchema::Unit);
        Ok(())
    }
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let mut builder = self.map.schema_builder();
        let result = seed.deserialize(&mut builder)?;
        let id = builder.id.unwrap();
        assert_eq!(id, *self.map.ids.get(&T::Value::maybe_type_id()).unwrap());
        self.enu = Some(VariantSchema::Newtype(id));
        Ok(result)
    }
    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut builder = self.map.seq_builder(len);
        let result = visitor.visit_seq(&mut builder)?;
        self.enu = Some(VariantSchema::Tuple(builder.seq));
        Ok(result)
    }
    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let mut builder = self.map.seq_builder(fields.len());
        let result = visitor.visit_seq(&mut builder)?;
        self.enu = Some(VariantSchema::Struct(
            fields
                .iter()
                .map(|x| x.to_string())
                .zip(builder.seq.into_iter())
                .collect(),
        ));
        Ok(result)
    }
}

#[test]
fn test_simple() {
    use itertools::Itertools;

    #[derive(Serialize, Deserialize)]
    enum Foo {
        FooUnit,
        FooNewtype(bool),
        FooTuple(bool, bool),
        FooStruct { x: bool, y: bool },
    }
    #[derive(Serialize, Deserialize)]
    struct Prim(
        bool,
        char,
        u8,
        u16,
        u32,
        u64,
        u128,
        i8,
        i16,
        i32,
        i64,
        i128,
        String,
        serde_bytes::ByteBuf,
    );

    #[derive(Serialize, Deserialize)]
    struct Struct {
        a: u8,
        b: u16,
    }

    #[derive(Serialize, Deserialize)]
    struct NewtypeStruct(Struct);

    let mut builder = SchemaMapBuilder::new();
    let foo_id = builder.add::<Foo>().unwrap();
    let vec_id = builder.add::<Vec<bool>>().unwrap();
    let map_id = builder.add::<HashMap<u8, u16>>().unwrap();
    let prim_id = builder.add::<Prim>().unwrap();
    let struct_id = builder.add::<Struct>().unwrap();
    let newtype_struct_id = builder.add::<NewtypeStruct>().unwrap();
    let bool_id = builder.get::<bool>();
    let char_id = builder.get::<char>();
    let string_id = builder.get::<String>();
    let bytes_id = builder.get::<serde_bytes::ByteBuf>();
    let u8_id = builder.get::<u8>();
    let u16_id = builder.get::<u16>();
    let u32_id = builder.get::<u32>();
    let u64_id = builder.get::<u64>();
    let u128_id = builder.get::<u128>();
    let i8_id = builder.get::<i8>();
    let i16_id = builder.get::<i16>();
    let i32_id = builder.get::<i32>();
    let i64_id = builder.get::<i64>();
    let i128_id = builder.get::<i128>();

    let schema = builder.build();
    let expected = SchemaMap {
        map: vec![
            (
                bool_id,
                SchemaType {
                    name: "bool".to_string(),
                    version: 0,
                    schema: Schema::Bool,
                },
            ),
            (
                char_id,
                SchemaType {
                    name: "char".to_string(),
                    version: 0,
                    schema: Schema::Char,
                },
            ),
            (
                string_id,
                SchemaType {
                    name: "alloc::string::String".to_string(),
                    version: 0,
                    schema: Schema::String,
                },
            ),
            (
                bytes_id,
                SchemaType {
                    name: "serde_bytes::bytebuf::ByteBuf".to_string(),
                    version: 0,
                    schema: Schema::Bytes,
                },
            ),
            (
                u8_id,
                SchemaType {
                    name: "u8".to_string(),
                    version: 0,
                    schema: Schema::Unsigned(8),
                },
            ),
            (
                u16_id,
                SchemaType {
                    name: "u16".to_string(),
                    version: 0,
                    schema: Schema::Unsigned(16),
                },
            ),
            (
                u32_id,
                SchemaType {
                    name: "u32".to_string(),
                    version: 0,
                    schema: Schema::Unsigned(32),
                },
            ),
            (
                u64_id,
                SchemaType {
                    name: "u64".to_string(),
                    version: 0,
                    schema: Schema::Unsigned(64),
                },
            ),
            (
                u128_id,
                SchemaType {
                    name: "u128".to_string(),
                    version: 0,
                    schema: Schema::Unsigned(128),
                },
            ),
            (
                i8_id,
                SchemaType {
                    name: "i8".to_string(),
                    version: 0,
                    schema: Schema::Signed(8),
                },
            ),
            (
                i16_id,
                SchemaType {
                    name: "i16".to_string(),
                    version: 0,
                    schema: Schema::Signed(16),
                },
            ),
            (
                i32_id,
                SchemaType {
                    name: "i32".to_string(),
                    version: 0,
                    schema: Schema::Signed(32),
                },
            ),
            (
                i64_id,
                SchemaType {
                    name: "i64".to_string(),
                    version: 0,
                    schema: Schema::Signed(64),
                },
            ),
            (
                i128_id,
                SchemaType {
                    name: "i128".to_string(),
                    version: 0,
                    schema: Schema::Signed(128),
                },
            ),
            (
                vec_id,
                SchemaType {
                    name: "alloc::vec::Vec<bool>".to_string(),
                    version: 0,
                    schema: Schema::Vec(bool_id),
                },
            ),
            (
                foo_id,
                SchemaType {
                    name: "serde_schema::builder::test_simple::Foo".to_string(),
                    version: 0,
                    schema: Schema::Enum {
                        enums: vec![
                            ("FooUnit".to_string(), VariantSchema::Unit),
                            ("FooNewtype".to_string(), VariantSchema::Newtype(bool_id)),
                            (
                                "FooTuple".to_string(),
                                VariantSchema::Tuple(vec![bool_id, bool_id]),
                            ),
                            (
                                "FooStruct".to_string(),
                                VariantSchema::Struct(vec![
                                    ("x".to_string(), bool_id),
                                    ("y".to_string(), bool_id),
                                ]),
                            ),
                        ],
                    },
                },
            ),
            (
                prim_id,
                SchemaType {
                    name: "serde_schema::builder::test_simple::Prim".to_string(),
                    version: 0,
                    schema: Schema::TupleStruct {
                        name: "Prim".to_string(),
                        fields: vec![
                            bool_id, char_id, u8_id, u16_id, u32_id, u64_id, u128_id, i8_id,
                            i16_id, i32_id, i64_id, i128_id, string_id, bytes_id,
                        ],
                    },
                },
            ),
            (
                struct_id,
                SchemaType {
                    name: "serde_schema::builder::test_simple::Struct".to_string(),
                    version: 0,
                    schema: Schema::Struct {
                        name: "Struct".to_string(),
                        fields: vec![("a".to_string(), u8_id), ("b".to_string(), u16_id)],
                    },
                },
            ),
            (
                newtype_struct_id,
                SchemaType {
                    name: "serde_schema::builder::test_simple::NewtypeStruct".to_string(),
                    version: 0,
                    schema: Schema::NewtypeStruct {
                        name: "NewtypeStruct".to_string(),
                        value: struct_id,
                    },
                },
            ),
            (
                map_id,
                SchemaType {
                    name: "std::collections::hash::map::HashMap<u8, u16>".to_string(),
                    version: 0,
                    schema: Schema::Map(u8_id, u16_id),
                },
            ),
        ]
        .into_iter()
        .collect(),
    };
    assert_eq!(
        schema.map.keys().collect::<HashSet<_>>(),
        expected.map.keys().collect::<HashSet<_>>()
    );
    for key in schema.map.keys() {
        let a = schema.get(*key);
        let b = expected.get(*key);
        assert_eq!(a, b, "{:#?}\n{:#?}", a, b);
    }
    assert_eq!(schema, expected);
}
