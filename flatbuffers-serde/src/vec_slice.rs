use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{Visitor, SeqAccess, MapAccess, EnumAccess};
use std::fmt::Formatter;

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd, Default)]
pub struct VecSlice {
    vec: Vec<u8>,
    head: usize,
}

impl VecSlice {
    pub fn new() -> Self {
        VecSlice { vec: vec![], head: 0 }
    }
    pub fn from_vec(vec: Vec<u8>, head: usize) -> Self {
        VecSlice { vec, head }
    }
    pub fn len(&self) -> usize {
        self.vec.len() - self.head
    }
    pub fn into_vec(self) -> Vec<u8> {
        self.vec
    }
    pub fn clear(&mut self) {
        self.vec.clear();
        self.vec.resize(self.vec.capacity(), 0);
        self.head = self.vec.len();
    }
    pub fn push_front(&mut self, extra: &[u8]) {
        let old_len = self.vec.len();
        let mut new_len = old_len;
        while self.head + (new_len - old_len) < extra.len() {
            if new_len == 0 {
                new_len = 1;
            } else {
                new_len *= 2;
            }
        }
        if new_len != old_len {
            self.vec.resize(new_len, 0);
            let (middle, back) = self.vec.split_at_mut(new_len - old_len);
            let front = &mut middle[0..old_len];
            back.copy_from_slice(front);
            front.fill(0);
            self.head += new_len - old_len;
        }
        let new_head = self.head - extra.len();
        self.vec[new_head..self.head].copy_from_slice(extra);
        self.head = new_head;
    }
}

impl AsRef<[u8]> for VecSlice {
    fn as_ref(&self) -> &[u8] {
        &self.vec[self.head..]
    }
}

impl Serialize for VecSlice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_bytes(&self.vec[self.head..])
    }
}

impl<'de> Deserialize<'de> for VecSlice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct Vis;
        impl<'de> Visitor<'de> for Vis {
            type Value = VecSlice;
            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                write!(formatter, "a byte buf")
            }
            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E> where E: serde::de::Error {
                Ok(VecSlice { vec: v, head: 0 })
            }
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> where E: serde::de::Error {
                Ok(VecSlice { vec: v.to_owned(), head: 0 })
            }
        }
        deserializer.deserialize_byte_buf(Vis)
    }
}

impl From<Vec<u8>> for VecSlice {
    fn from(vec: Vec<u8>) -> Self {
        VecSlice { vec, head: 0 }
    }
}

#[test]
fn test_push() {
    let mut slice = VecSlice::new();
    slice.push_front(&[0]);
    slice.push_front(&[2, 1]);
    slice.push_front(&[4, 3]);
    slice.push_front(&[8, 7, 6, 5]);
    assert_eq!(slice.as_ref(), &[8, 7, 6, 5, 4, 3, 2, 1, 0]);
}