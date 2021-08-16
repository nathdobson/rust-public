#![feature(specialization)]
#![feature(never_type)]
#![feature(box_syntax)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(incomplete_features)]

mod error;
mod builder;

use serde::{Serialize, Deserializer};
use serde::Deserialize;
use serde::de::{Visitor, SeqAccess, DeserializeSeed, EnumAccess, VariantAccess, IntoDeserializer};
use std::fmt::{Display, Debug, Formatter};
use std::collections::HashMap;
use std::any::{TypeId, Any};
use std::marker::PhantomData;
use std::collections::hash_map::Entry;

use crate::error::Error;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone)]
pub enum VariantSchema {
    Unit,
    Newtype(SchemaId),
    Tuple(Vec<SchemaId>),
    Struct(Vec<(String, SchemaId)>),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone)]
pub enum Schema {
    Bool,
    Unsigned(usize),
    Signed(usize),
    Float(usize),
    Char,
    String,
    Bytes,
    Option(SchemaId),
    Unit,
    UnitStruct { name: String },
    NewtypeStruct { name: String, value: SchemaId },
    Enum { enums: Vec<(String, VariantSchema)> },
    Vec(SchemaId),
    Map(SchemaId, SchemaId),
    Tuple(Vec<SchemaId>),
    TupleStruct { name: String, fields: Vec<SchemaId> },
    Struct { name: String, fields: Vec<(String, SchemaId)> },
}

#[derive(Serialize, Deserialize, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct SchemaType {
    name: String,
    version: usize,
    schema: Schema,
}

#[derive(Serialize, Deserialize, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Copy)]
pub struct SchemaId(u64);

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct SchemaMap {
    map: HashMap<SchemaId, SchemaType>,
}

impl SchemaMap {
    fn get(&self, id: SchemaId) -> &SchemaType {
        self.map.get(&id).unwrap()
    }
}