use serde::{Serialize, Deserialize};
use serde::ser::{SerializeSeq, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant, SerializeMap, SerializeStruct, SerializeStructVariant};
use flatbuffers::{FlatBufferBuilder, Push, UnionWIPOffset, WIPOffset, VOffsetT, UOffsetT};
use std::fmt::{Display, Formatter, Debug};
use std::error::Error;
use lazy_static::lazy_static;
use crate::{EmptyPush, U128, VariantT};

pub struct Stack {
    field_stack: Vec<Value>,
    vector_stack: Vec<OneValue>,
}

#[derive(Copy, Clone)]
pub enum OneValue {
    Ref(WIPOffset<UnionWIPOffset>),
    SomeRef(WIPOffset<UnionWIPOffset>),
    NoneRef,
    Fixed0,
    Fixed8(u8),
    Fixed16(u16),
    Fixed32(u32),
    Fixed64(u64),
    Fixed128(u128),
}

#[derive(Copy, Clone, Debug)]
pub enum Value {
    OneValue(OneValue),
    Enum {
        variant: VariantT,
        value: OneValue,
    },
}

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

impl Debug for OneValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OneValue::Ref(x) =>
                f.debug_tuple("OneValue::Ref")
                    .field(&x.value())
                    .finish(),
            OneValue::SomeRef(x) =>
                f.debug_tuple("OneValue::SomeRef")
                    .field(&x.value())
                    .finish(),
            OneValue::NoneRef => f.debug_struct("OneValue::NoneRef").finish(),
            OneValue::Fixed0 => f.debug_struct("OneValue::Fixed0").finish(),
            OneValue::Fixed8(x) =>
                f.debug_tuple("OneValue::Fixed8").field(&x).finish(),
            OneValue::Fixed16(x) =>
                f.debug_tuple("OneValue::Fixed16").field(&x).finish(),
            OneValue::Fixed32(x) =>
                f.debug_tuple("OneValue::Fixed32").field(&x).finish(),
            OneValue::Fixed64(x) =>
                f.debug_tuple("OneValue::Fixed64").field(&x).finish(),
            OneValue::Fixed128(x) =>
                f.debug_tuple("OneValue::Fixed128").field(&x).finish(),
        }
    }
}

impl Stack {
    pub fn new() -> Self {
        Stack { field_stack: vec![], vector_stack: vec![] }
    }
}

fn variant_tag(x: u32) -> u16 {
    x as u16
}

impl OneValue {
    fn push_slot_always(self, fbb: &mut FlatBufferBuilder, off: VOffsetT) {
        match self {
            OneValue::Ref(x) => fbb.push_slot_always(off, x),
            OneValue::SomeRef(x) => fbb.push_slot_always(off, x),
            OneValue::NoneRef => {}
            OneValue::Fixed0 => todo!(),
            OneValue::Fixed8(x) => fbb.push_slot(off, x, 0),
            OneValue::Fixed16(x) => fbb.push_slot(off, x, 0),
            OneValue::Fixed32(x) => fbb.push_slot(off, x, 0),
            OneValue::Fixed64(_) => todo!(),
            OneValue::Fixed128(_) => todo!(),
        }
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
    fn start_table(self) -> TableBuilder<'a, 'b> {
        let len = self.stack.field_stack.len();
        TableBuilder { serializer: self, element_start: len }
    }
    fn value_to_offset(&mut self, value: Value) -> WIPOffset<UnionWIPOffset> {
        let value = self.value_to_one_value(value);
        let value = self.one_value_to_offset(value);
        value
    }
    fn value_to_one_value(&mut self, value: Value) -> OneValue {
        match value {
            Value::OneValue(x) => x,
            Value::Enum { variant, value } => {
                let builder = self.reborrow().start_table();
                builder.serializer.stack.field_stack.push(Value::Enum { variant, value });
                OneValue::Ref(builder.end_table())
            }
        }
    }

    fn one_value_to_offset(&mut self, value: OneValue) -> WIPOffset<UnionWIPOffset> {
        println!("one_value_to_offset({:?})", value);
        match value {
            OneValue::Ref(x) => x,
            OneValue::SomeRef(x) => self.fbb.push(x).as_union_value(),
            OneValue::NoneRef => self.fbb.push(0 as UOffsetT).as_union_value(),
            OneValue::Fixed0 => todo!(),
            OneValue::Fixed8(x) => self.fbb.push(x).as_union_value(),
            OneValue::Fixed16(_) => todo!(),
            OneValue::Fixed32(_) => todo!(),
            OneValue::Fixed64(_) => todo!(),
            OneValue::Fixed128(_) => todo!(),
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
    fn end_table(self) -> WIPOffset<UnionWIPOffset> {
        let table = self.serializer.fbb.start_table();
        let mut off = 4;
        for element in self.serializer.stack.field_stack.drain(self.element_start..) {
            match element {
                Value::OneValue(element) => {
                    element.push_slot_always(self.serializer.fbb, off);
                    off += 2;
                }
                Value::Enum { variant, value } => {
                    self.serializer.fbb.push_slot_always(off, variant);
                    off += 2;
                    value.push_slot_always(self.serializer.fbb, off);
                    off += 2;
                }
            }
        }
        let table = self.serializer.fbb.end_table(table).as_union_value();
        table
    }
}

impl<'a, 'b> SerializeSeq for VectorBuilder<'a, 'b> {
    type Ok = Value;
    type Error = SerializeError;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where T: Serialize {
        let value = value.serialize(self.serializer.reborrow())?;
        let value = self.serializer.value_to_one_value(value);
        self.serializer.stack.vector_stack.push(value);
        Ok(())
    }
    fn end(self) -> Result<Value> {
        let mut iter = self.serializer.stack.vector_stack.drain(self.element_start..).peekable();
        let len = iter.len();
        let head = iter.peek().cloned();
        if let Some(head) = &head {
            match head {
                OneValue::Ref(_) => { self.serializer.fbb.start_vector::<UOffsetT>(len); }
                OneValue::SomeRef(_) => { self.serializer.fbb.start_vector::<UOffsetT>(len); }
                OneValue::NoneRef => { self.serializer.fbb.start_vector::<UOffsetT>(len); }
                OneValue::Fixed0 => { self.serializer.fbb.start_vector::<EmptyPush>(len); }
                OneValue::Fixed8(_) => { self.serializer.fbb.start_vector::<u8>(len); }
                OneValue::Fixed16(_) => { self.serializer.fbb.start_vector::<u16>(len); }
                OneValue::Fixed32(_) => { self.serializer.fbb.start_vector::<u32>(len); }
                OneValue::Fixed64(_) => { self.serializer.fbb.start_vector::<u64>(len); }
                OneValue::Fixed128(_) => { self.serializer.fbb.start_vector::<U128>(len); }
            }
        } else {
            self.serializer.fbb.start_vector::<U128>(len);
        }
        for element in iter.rev() {
            match element {
                OneValue::Ref(x) => { self.serializer.fbb.push(x); }
                OneValue::SomeRef(x) => { self.serializer.fbb.push(x); }
                OneValue::NoneRef => { self.serializer.fbb.push(0 as UOffsetT); }
                OneValue::Fixed0 => todo!(),
                OneValue::Fixed8(x) => { self.serializer.fbb.push(x); }
                OneValue::Fixed16(x) => { self.serializer.fbb.push(x); }
                OneValue::Fixed32(x) => { self.serializer.fbb.push(x); }
                OneValue::Fixed64(_) => todo!(),
                OneValue::Fixed128(x) => { self.serializer.fbb.push(U128(x)); }
            }
        }
        let vector = if let Some(head) = &head {
            match head {
                OneValue::Ref(_) => self.serializer.fbb.end_vector::<UOffsetT>(len).as_union_value(),
                OneValue::SomeRef(_) => self.serializer.fbb.end_vector::<UOffsetT>(len).as_union_value(),
                OneValue::NoneRef => self.serializer.fbb.end_vector::<UOffsetT>(len).as_union_value(),
                OneValue::Fixed0 => self.serializer.fbb.end_vector::<EmptyPush>(len).as_union_value(),
                OneValue::Fixed8(_) => self.serializer.fbb.end_vector::<u8>(len).as_union_value(),
                OneValue::Fixed16(_) => self.serializer.fbb.end_vector::<u16>(len).as_union_value(),
                OneValue::Fixed32(_) => self.serializer.fbb.end_vector::<u32>(len).as_union_value(),
                OneValue::Fixed64(_) => todo!(),
                OneValue::Fixed128(_) => self.serializer.fbb.end_vector::<U128>(len).as_union_value(),
            }
        } else {
            self.serializer.fbb.end_vector::<U128>(len).as_union_value()
        };
        Ok(Value::OneValue(OneValue::Ref(vector)))
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
        Ok(Value::OneValue(OneValue::Ref(self.end_table())))
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
        Ok(Value::OneValue(OneValue::Fixed8(if v { 1 } else { 0 })))
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
    fn serialize_i128(mut self, v: i128) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_u8(mut self, v: u8) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed8(v)))
    }
    fn serialize_u16(mut self, v: u16) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed16(v)))
    }
    fn serialize_u32(mut self, v: u32) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_u64(mut self, v: u64) -> Result<Self::Ok> {
        todo!()
    }
    fn serialize_u128(mut self, v: u128) -> Result<Self::Ok> {
        Ok(Value::OneValue(OneValue::Fixed128(v)))
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
        Ok(Value::OneValue(OneValue::NoneRef))
    }
    fn serialize_some<T: ?Sized>(mut self, value: &T) -> Result<Self::Ok> where T: Serialize {
        let value = value.serialize(self.reborrow())?;
        println!("Serializing Some({:?})", value);
        let value = Value::OneValue(OneValue::SomeRef(self.value_to_offset(value)));
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
        let mut table = self.start_table();
        table.serialize_element(value)?;
        Ok(Value::OneValue(OneValue::Ref(table.end_table())))
    }
    fn serialize_newtype_variant<T: ?Sized>(mut self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<Value> where T: Serialize {
        let value = value.serialize(self.reborrow())?;
        let value = self.value_to_one_value(value);
        Ok(Value::Enum { variant: variant_index as u16, value })
    }
    fn serialize_seq(mut self, len: Option<usize>) -> Result<VectorBuilder<'a, 'b>> {
        self.start_vector()
    }
    fn serialize_tuple(mut self, len: usize) -> Result<TableBuilder<'a, 'b>> {
        Ok(self.start_table())
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