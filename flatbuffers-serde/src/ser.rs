use serde::{Serialize, Deserialize};
use serde::ser::{SerializeSeq, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, SerializeMap, SerializeStruct, SerializeStructVariant};
use flatbuffers::{FlatBufferBuilder, Push, UnionWIPOffset, WIPOffset, VOffsetT, UOffsetT};
use std::fmt::{Display, Formatter, Debug};
use std::error::Error;
use lazy_static::lazy_static;

pub struct Stack {
    field_stack: Vec<Value>,
    vector_stack: Vec<Value>,
}
//
// #[derive(Debug)]
// pub enum Value {
//     Value(Value),
//     Some(Value),
//     None,
// }

pub enum Value {
    Ref(WIPOffset<UnionWIPOffset>),
    SomeRef(WIPOffset<UnionWIPOffset>),
    NoneRef,
    Fixed0,
    Fixed8(u8),
    Fixed16(u16),
    Fixed32(u32),
    Fixed64(u64),
    Fixed128(u128),
    Enum {
        variant: u32,
        value: WIPOffset<UnionWIPOffset>,
    },
}

struct EmptyPush;

pub struct Serializer<'a, 'b> {
    fbb: &'a mut FlatBufferBuilder<'b>,
    stack: &'a mut Stack,
}

pub struct VectorBuilder<'a, 'b> {
    serializer: Serializer<'a, 'b>,
    element_start: usize,
}

pub struct TableBuilder<'a, 'b> {
    serializer: Serializer<'a, 'b>,
    element_start: usize,
}

impl Push for EmptyPush {
    type Output = ();
    fn push(&self, dst: &mut [u8], _rest: &[u8]) {}
}

#[derive(Debug)]
pub enum SerializeError {
    Custom(String)
}

impl std::error::Error for SerializeError {}

impl Display for SerializeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl serde::ser::Error for SerializeError {
    fn custom<T>(msg: T) -> Self where T: Display {
        SerializeError::Custom(msg.to_string())
    }
}

type Result<T> = std::result::Result<T, SerializeError>;

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Ref(x) =>
                f.debug_tuple("Value::Ref")
                    .field(&x.value())
                    .finish(),
            Value::SomeRef(x) =>
                f.debug_tuple("Value::SomeRef")
                    .field(&x.value())
                    .finish(),
            Value::NoneRef => f.debug_struct("Value::NoneRef").finish(),
            Value::Fixed0 => f.debug_struct("Value::Fixed0").finish(),
            Value::Fixed8(x) =>
                f.debug_tuple("Value::Fixed8").field(&x).finish(),
            Value::Fixed16(x) =>
                f.debug_tuple("Value::Fixed16").field(&x).finish(),
            Value::Fixed32(x) =>
                f.debug_tuple("Value::Fixed32").field(&x).finish(),
            Value::Fixed64(x) =>
                f.debug_tuple("Value::Fixed64").field(&x).finish(),
            Value::Fixed128(x) =>
                f.debug_tuple("Value::Fixed128").field(&x).finish(),
            Value::Enum { variant, value } =>
                f.debug_struct("Value::Enum")
                    .field("variant", &variant)
                    .field("value", &value.value())
                    .finish(),
        }
    }
}

impl Stack {
    pub fn new() -> Self {
        Stack { field_stack: vec![], vector_stack: vec![] }
    }
}

impl<'a, 'b> Serializer<'a, 'b> {
    pub fn new(fbb: &'a mut FlatBufferBuilder<'b>, stack: &'a mut Stack) -> Self {
        Serializer { fbb, stack }
    }
    fn reborrow<'c>(&'c mut self) -> Serializer<'c, 'b> {
        Serializer { fbb: self.fbb, stack: self.stack }
    }
    fn start_vector(mut self) -> Result<VectorBuilder<'a, 'b>> {
        let len = self.stack.vector_stack.len();
        Ok(VectorBuilder {
            serializer: self,
            element_start: len,
        })
    }
    fn start_table(self) -> Result<TableBuilder<'a, 'b>> {
        let len = self.stack.field_stack.len();
        Ok(TableBuilder { serializer: self, element_start: len })
    }
    fn value_to_offset(&mut self, value: Value) -> WIPOffset<UnionWIPOffset> {
        match value {
            Value::Ref(x) => x,
            Value::SomeRef(x) => self.fbb.push(x).as_union_value(),
            Value::NoneRef => self.fbb.push(0 as UOffsetT).as_union_value(),
            Value::Fixed0 => todo!(),
            Value::Fixed8(x) => self.fbb.push(x).as_union_value(),
            Value::Fixed16(_) => todo!(),
            Value::Fixed32(_) => todo!(),
            Value::Fixed64(_) => todo!(),
            Value::Fixed128(_) => todo!(),
            Value::Enum { .. } => todo!(),
        }
    }
    // fn value_option_to_offset(&mut self, value: Value) -> WIPOffset<UnionWIPOffset> {
    //     println!("value_option_to_offset({:?})", value);
    //     match value {
    //         Value::Value(value) => match value {
    //             Value::Offset(x) => x,
    //             Value::Fixed0 => self.fbb.push(EmptyPush).as_union_value(),
    //             Value::Fixed8(x) => self.fbb.push(x).as_union_value(),
    //             Value::Fixed16(_) => todo!(),
    //             Value::Fixed32(x) => self.fbb.push(x).as_union_value(),
    //             Value::Fixed64(_) => todo!(),
    //             Value::Fixed128(_) => todo!(),
    //             Value::Enum { .. } => todo!(),
    //             Value::Null => todo!()
    //         },
    //         Value::Some(x) => {
    //             let offset = self.value_to_box(x);
    //             println!("offset = {:?}", offset.value());
    //             self.fbb.push(offset).as_union_value()
    //         }
    //         Value::None => self.fbb.push(0 as UOffsetT).as_union_value()
    //     }
    // }
    // fn value_option_to_value(&mut self, value: Value) -> Value {
    //     match value {
    //         Value::Value(x) => x,
    //         Value::Some(x) =>
    //             Value::Offset(self.value_to_box(x)),
    //         Value::None =>
    //             Value::Null
    //     }
    // }
    // fn value_to_box(&mut self, value: Value) -> WIPOffset<UnionWIPOffset> {
    //     match value {
    //         Value::Offset(x) => self.fbb.push(x).as_union_value(),
    //         Value::Fixed0 => self.fbb.push(EmptyPush).as_union_value(),
    //         Value::Fixed8(x) => self.fbb.push(x).as_union_value(),
    //         Value::Fixed16(_) => todo!(),
    //         Value::Fixed32(x) => self.fbb.push(x).as_union_value(),
    //         Value::Fixed64(_) => todo!(),
    //         Value::Fixed128(_) => todo!(),
    //         Value::Enum { .. } => todo!(),
    //         Value::Null => self.fbb.push(0 as UOffsetT).as_union_value()
    //     }
    // }
    pub fn serialize_to_offset<T: Serialize>(&mut self, v: &T) -> Result<WIPOffset<UnionWIPOffset>> {
        let value = v.serialize(self.reborrow())?;
        Ok(self.value_to_offset(value))
    }
}

impl<'a, 'b> TableBuilder<'a, 'b> {
    fn end_table(self) -> Result<Value> {
        let table = self.serializer.fbb.start_table();
        for (index, element) in self.serializer.stack.field_stack.drain(self.element_start..).enumerate() {
            let off = (index * 2 + 4) as VOffsetT;
            println!("Ending table with {:?} {:?}", index, element);
            println!("head {:?}", self.serializer.fbb.push(EmptyPush));
            match element {
                Value::Ref(x) => self.serializer.fbb.push_slot_always(off, x),
                Value::SomeRef(x) => self.serializer.fbb.push_slot_always(off, x),
                Value::NoneRef => {}
                Value::Fixed0 => todo!(),
                Value::Fixed8(x) => self.serializer.fbb.push_slot(off, x, 0),
                Value::Fixed16(x) => self.serializer.fbb.push_slot(off, x, 0),
                Value::Fixed32(x) => self.serializer.fbb.push_slot(off, x, 0),
                Value::Fixed64(_) => todo!(),
                Value::Fixed128(_) => todo!(),
                Value::Enum { .. } => todo!(),
            }
            println!("head {:?}", self.serializer.fbb.push(EmptyPush));
        }
        let table = self.serializer.fbb.end_table(table).as_union_value();
        Ok(Value::Ref(table))
    }
}

impl<'a, 'b> SerializeSeq for VectorBuilder<'a, 'b> {
    type Ok = Value;
    type Error = SerializeError;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        let value = value.serialize(self.serializer.reborrow())?;
        //let value = self.serializer.value_option_to_value(value);
        self.serializer.stack.vector_stack.push(value);
        Ok(())
    }
    fn end(self) -> Result<Value> {
        todo!()
    }
}

impl<'a, 'b> SerializeTuple for TableBuilder<'a, 'b> {
    type Ok = Value;
    type Error = SerializeError;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        let value = value.serialize(self.serializer.reborrow())?;
        println!("element {:?}", value);
        println!("head {:?}", self.serializer.fbb.push(EmptyPush));
        self.serializer.stack.field_stack.push(value);
        Ok(())
    }
    fn end(mut self) -> Result<Value> {
        Ok(self.end_table()?)
    }
}

impl<'a, 'b> SerializeTupleStruct for TableBuilder<'a, 'b> {
    type Ok = Value;
    type Error = SerializeError;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        todo!()
    }
    fn end(self) -> Result<Value> { todo!() }
}

impl<'a, 'b> SerializeTupleVariant for TableBuilder<'a, 'b> {
    type Ok = Value;
    type Error = SerializeError;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        todo!()
    }
    fn end(self) -> Result<Value> { todo!() }
}

impl<'a, 'b> SerializeMap for VectorBuilder<'a, 'b> {
    type Ok = Value;
    type Error = SerializeError;
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()> where T: Serialize {
        todo!()
    }
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        todo!()
    }
    fn end(self) -> Result<Value> { todo!() }
}

impl<'a, 'b> SerializeStruct for TableBuilder<'a, 'b> {
    type Ok = Value;
    type Error = SerializeError;
    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> where T: Serialize {
        todo!()
    }
    fn end(self) -> Result<Value> { todo!() }
}

impl<'a, 'b> SerializeStructVariant for TableBuilder<'a, 'b> {
    type Ok = Value;
    type Error = SerializeError;
    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> where T: Serialize {
        todo!()
    }
    fn end(self) -> Result<Value> { todo!() }
}

impl<'a, 'b> serde::Serializer for Serializer<'a, 'b> {
    type Ok = Value;
    type Error = SerializeError;
    type SerializeSeq = VectorBuilder<'a, 'b>;
    type SerializeTuple = TableBuilder<'a, 'b>;
    type SerializeTupleStruct = TableBuilder<'a, 'b>;
    type SerializeTupleVariant = TableBuilder<'a, 'b>;
    type SerializeMap = VectorBuilder<'a, 'b>;
    type SerializeStruct = TableBuilder<'a, 'b>;
    type SerializeStructVariant = TableBuilder<'a, 'b>;
    fn serialize_bool(mut self, v: bool) -> Result<Self::Ok> {
        Ok(Value::Fixed8(if v { 1 } else { 0 }))
    }
    fn serialize_i8(mut self, v: i8) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_i16(mut self, v: i16) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_i32(mut self, v: i32) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_i64(mut self, v: i64) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_u8(mut self, v: u8) -> Result<Self::Ok> {
        Ok(Value::Fixed8(v))
    }
    fn serialize_u16(mut self, v: u16) -> Result<Self::Ok> {
        Ok(Value::Fixed16(v))
    }
    fn serialize_u32(mut self, v: u32) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_u64(mut self, v: u64) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_f32(mut self, v: f32) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_f64(mut self, v: f64) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_char(mut self, v: char) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_str(mut self, v: &str) -> Result<Self::Ok> {
        self.serialize_bytes(v.as_bytes())
    }
    fn serialize_bytes(mut self, v: &[u8]) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_none(mut self) -> Result<Self::Ok> {
        Ok(Value::NoneRef)
    }
    fn serialize_some<T: ?Sized>(mut self, value: &T) -> Result<Self::Ok> where T: Serialize {
        let value = value.serialize(self.reborrow())?;
        println!("Serializing Some({:?})", value);
        let value = Value::SomeRef(self.value_to_offset(value));
        println!("To {:?}", value);
        Ok(value)
    }
    fn serialize_unit(mut self) -> Result<Value> {
        todo!()
    }
    fn serialize_unit_struct(mut self, name: &'static str) -> Result<Value> {
        todo!()
    }
    fn serialize_unit_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<Value> {
        self.serialize_u32(variant_index)?;
        todo!()
    }
    fn serialize_newtype_struct<T: ?Sized>(mut self, name: &'static str, value: &T) -> Result<Value> where T: Serialize {
        let mut table = self.start_table()?;
        table.serialize_element(value)?;
        Ok(table.end_table()?)
    }
    fn serialize_newtype_variant<T: ?Sized>(mut self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<Value> where T: Serialize {
        // self.reborrow().serialize_u32(variant_index)?;
        // value.serialize(self)?;
        todo!()
    }
    fn serialize_seq(mut self, len: Option<usize>) -> Result<VectorBuilder<'a, 'b>> {
        //self.serialize_counted()
        todo!()
    }
    fn serialize_tuple(mut self, len: usize) -> Result<TableBuilder<'a, 'b>> {
        self.start_table()
    }
    fn serialize_tuple_struct(mut self, name: &'static str, len: usize) -> Result<TableBuilder<'a, 'b>> {
        //Ok(self)
        todo!()
    }
    fn serialize_tuple_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<TableBuilder<'a, 'b>> {
        // self.reborrow().serialize_u32(variant_index)?;
        // Ok(self)
        todo!()
    }
    fn serialize_map(mut self, len: Option<usize>) -> Result<VectorBuilder<'a, 'b>> {
        // self.serialize_counted()
        todo!()
    }
    fn serialize_struct(mut self, name: &'static str, len: usize) -> Result<TableBuilder<'a, 'b>> {
        //Ok(self)
        todo!()
    }
    fn serialize_struct_variant(mut self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<TableBuilder<'a, 'b>> {
        // self.reborrow().serialize_u32(variant_index)?;
        // Ok(self)
        todo!()
    }
}