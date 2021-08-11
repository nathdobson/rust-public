use serde::Serialize;
use serde::Deserialize;
use std::collections::HashMap;
use crate::any::AnySerde;
use std::ops::Deref;
use std::any::{Any, TypeId};
use std::marker::PhantomData;
use crate::binary::{Error};

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
    value.serialize(crate::binary::ser::BinarySerializer::new(&mut vec)).unwrap();
}
