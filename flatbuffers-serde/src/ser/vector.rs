use flatbuffers::UOffsetT;
use serde::ser::SerializeSeq;
use serde::Serialize;

use crate::flat_util::{Flat128, FlatUnit};
use crate::ser::error::Error;
use crate::ser::value::{OneValue, Value};
use crate::ser::wrapper::Serializer;
use crate::ser::{error, Result};

pub struct VectorBuilder<'a, 'b> {
    serializer: Serializer<'a, 'b>,
    element_start: usize,
}

impl<'a, 'b> VectorBuilder<'a, 'b> {
    pub fn new(serializer: Serializer<'a, 'b>) -> Self {
        let len = serializer.stack.vector_stack.len();
        VectorBuilder {
            serializer,
            element_start: len,
        }
    }
    pub fn reborrow<'c>(&'c mut self) -> Serializer<'c, 'b> { self.serializer.reborrow() }
    pub fn push(&mut self, value: OneValue) { self.serializer.stack.vector_stack.push(value); }
}

impl<'a, 'b> VectorBuilder<'a, 'b> {
    fn end_vector(self) -> Result<Value> {
        let mut iter = self
            .serializer
            .stack
            .vector_stack
            .drain(self.element_start..)
            .peekable();
        let len = iter.len();
        let head = iter.peek().cloned();
        if let Some(head) = &head {
            match head {
                OneValue::Ref(_) => {
                    self.serializer.fbb.start_vector::<UOffsetT>(len);
                }
                OneValue::SomeRef(_) => {
                    self.serializer.fbb.start_vector::<UOffsetT>(len);
                }
                OneValue::NoneRef => {
                    self.serializer.fbb.start_vector::<UOffsetT>(len);
                }
                OneValue::Fixed0 => {
                    self.serializer.fbb.start_vector::<FlatUnit>(len);
                }
                OneValue::Fixed8(_) => {
                    self.serializer.fbb.start_vector::<u8>(len);
                }
                OneValue::Fixed16(_) => {
                    self.serializer.fbb.start_vector::<u16>(len);
                }
                OneValue::Fixed32(_) => {
                    self.serializer.fbb.start_vector::<u32>(len);
                }
                OneValue::Fixed64(_) => {
                    self.serializer.fbb.start_vector::<u64>(len);
                }
                OneValue::Fixed128(_) => {
                    self.serializer.fbb.start_vector::<Flat128>(len);
                }
            }
        } else {
            self.serializer.fbb.start_vector::<Flat128>(len);
        }
        for element in iter.rev() {
            match element {
                OneValue::Ref(x) => {
                    self.serializer.fbb.push(x);
                }
                OneValue::SomeRef(x) => {
                    self.serializer.fbb.push(x);
                }
                OneValue::NoneRef => {
                    self.serializer.fbb.push(0 as UOffsetT);
                }
                OneValue::Fixed0 => {
                    self.serializer.fbb.push(FlatUnit);
                }
                OneValue::Fixed8(x) => {
                    self.serializer.fbb.push(x);
                }
                OneValue::Fixed16(x) => {
                    self.serializer.fbb.push(x);
                }
                OneValue::Fixed32(x) => {
                    self.serializer.fbb.push(x);
                }
                OneValue::Fixed64(x) => {
                    self.serializer.fbb.push(x);
                }
                OneValue::Fixed128(x) => {
                    self.serializer.fbb.push(Flat128(x));
                }
            }
        }
        let vector = if let Some(head) = &head {
            match head {
                OneValue::Ref(_) => self
                    .serializer
                    .fbb
                    .end_vector::<UOffsetT>(len)
                    .as_union_value(),
                OneValue::SomeRef(_) => self
                    .serializer
                    .fbb
                    .end_vector::<UOffsetT>(len)
                    .as_union_value(),
                OneValue::NoneRef => self
                    .serializer
                    .fbb
                    .end_vector::<UOffsetT>(len)
                    .as_union_value(),
                OneValue::Fixed0 => self
                    .serializer
                    .fbb
                    .end_vector::<FlatUnit>(len)
                    .as_union_value(),
                OneValue::Fixed8(_) => self.serializer.fbb.end_vector::<u8>(len).as_union_value(),
                OneValue::Fixed16(_) => self.serializer.fbb.end_vector::<u16>(len).as_union_value(),
                OneValue::Fixed32(_) => self.serializer.fbb.end_vector::<u32>(len).as_union_value(),
                OneValue::Fixed64(_) => self.serializer.fbb.end_vector::<u64>(len).as_union_value(),
                OneValue::Fixed128(_) => self
                    .serializer
                    .fbb
                    .end_vector::<Flat128>(len)
                    .as_union_value(),
            }
        } else {
            self.serializer
                .fbb
                .end_vector::<Flat128>(len)
                .as_union_value()
        };
        Ok(Value::OneValue(OneValue::Ref(vector)))
    }
}

impl<'a, 'b> SerializeSeq for VectorBuilder<'a, 'b> {
    type Ok = Value;
    type Error = Error;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let value = value.serialize(self.serializer.reborrow())?;
        let value = value.to_one_value(&mut self.serializer);
        self.serializer.stack.vector_stack.push(value);
        Ok(())
    }
    fn end(self) -> Result<Value> { self.end_vector() }
}
