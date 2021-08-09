use crate::{binary};
use serde::Serialize;
use serde::Deserialize;
use std::collections::HashMap;
use bincode::Options;
use crate::any::AnySerde;
use std::ops::Deref;
use std::any::{Any, TypeId};
use std::marker::PhantomData;
use crate::ser::{AnySerialize};
use crate::binary::{BinarySerializer, Error};
use crate::ser::binary::AnySerializeBinary;

#[derive(Serialize, Deserialize)]
struct Test1;

#[derive(Serialize, Deserialize)]
struct Test2(i8);

#[derive(Serialize, Deserialize)]
struct Test3(i16, i32);

#[derive(Serialize, Deserialize)]
struct Test4 {
    x: i64,
    y: u64,
}

#[derive(Serialize, Deserialize)]
enum Test5 { A }

#[derive(Serialize, Deserialize)]
enum Test6 { A(i8) }

#[derive(Serialize, Deserialize)]
enum Test7 { A(i16, i32) }

#[derive(Serialize, Deserialize)]
enum Test8 { A { x: i64, y: i64 } }

type Test9 = Option<u128>;

type Test10 = Vec<u8>;

type Test11 = String;

type Test12 = Vec<i16>;

type Test13 = HashMap<i32, i64>;

type Test14 = ();

type Test15 = (i32, );

type Test16 = (i32, i64);

#[derive(Serialize, Deserialize)]
struct Test(
    Test1, Test2, Test3, Test4, Test5, Test6, Test7, Test8,
    Test9, Test10, Test11, Test12, Test13, Test14, Test15, Test16);

#[test]
fn test_binary() {
    let mut vec = vec![];
    let value = Test(
        Test1,
        Test2(1),
        Test3(2, 3),
        Test4 { x: 4, y: 5 },
        Test5::A,
        Test6::A(6),
        Test7::A(7, 8),
        Test8::A { x: 9, y: 10 },
        Some(11),
        vec![12, 13],
        "asd".to_string(),
        vec![14, 15],
        vec![(16, 17), (18, 19)].into_iter().collect(),
        (),
        (20, ),
        (21, 22),
    );
    value.serialize(crate::binary::BinarySerializer::new(&mut vec)).unwrap();
}


//impl_any_serde!(Any32, "serde_any::test::Any32");

// static ANY32_IMPL: AnySerializeEntry<Any32> = AnySerializeEntry::new();
// static ANY32_IMPL_REF: &'static AnySerializeEntry<Any32> = &ANY32_IMPL;
// static ANY32_IMPL_BINARY: &'static dyn AnySerializeBinary = &ANY32_IMPL_REF;
// static ANY32_IMPL_JSON: &'static dyn AnySerializeJson = &ANY32_IMPL_REF;
//
// impl AnySerialize for Any32 {
//     fn as_any(&self) -> &dyn Any { self }
//     fn get_any_serialize_impl(&self, key: AnySerializeKey) -> Option<&'static dyn Any> {
//         if key == *ANY_SERIALIZE_BINARY_KEY {
//             Some(&ANY32_IMPL_BINARY)
//         } else if key == *ANY_SERIALIZE_JSON_KEY {
//             Some(&ANY32_IMPL_JSON)
//         } else {
//             None
//         }
//     }
// }
//
// #[test]
// fn test_binary_any() {
//     let encoded = binary::serialize(&AnySerde::new(Any32(10))).unwrap();
//     assert_eq!(vec![162, 84, 184, 227, 172, 2, 145, 44,
//                     181, 48, 53, 118, 52, 245, 232, 222,
//                     5, 146, 159, 249, 183, 58, 17, 251,
//                     82, 96, 233, 238, 56, 59, 91, 37,
//                     4, 0, 0, 0, 0, 0, 0, 0,
//                     10, 0, 0, 0], encoded);
//     assert_eq!(&Any32(10), binary::deserialize::<AnySerde>(
//         &encoded).unwrap().deref().as_any().downcast_ref::<Any32>().unwrap());
// }
//
// #[test]
// fn test_json_any() {
//     let encoded = serde_json::to_string(&AnySerde::new(Any32(10))).unwrap();
//     assert_eq!(r#"{"serde_any::test::Any32":10}"#, encoded);
//     assert_eq!(&Any32(10), serde_json::from_slice::<AnySerde>(encoded.as_bytes()).unwrap().deref().as_any().downcast_ref::<Any32>().unwrap());
// }