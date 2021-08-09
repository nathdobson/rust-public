use crate::ser::{AnySerializer, AnySerialize};

type JsonSerializer<'b> = serde_json::Serializer<&'b mut Vec<u8>>;

impl<'a, 'b> AnySerializer for &'a mut JsonSerializer<'b> {
    fn serialize_dyn(self, value: &dyn AnySerialize) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}