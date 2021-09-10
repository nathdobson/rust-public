#![deny(unused_must_use)]
#![allow(unused_imports)]

use flatbuffers::{FlatBufferBuilder, root};
use flatbuffers_serde::any_generated::AnyFlat;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use flatbuffers_serde::any_raw_generated::AnyFlatRaw;

fn run_test<T: Serialize + for<'de> Deserialize<'de> + Eq + Debug>(value: T, expected: &[u8]) {
    println!("\nSerializing {:?}", value);
    let mut fbb = FlatBufferBuilder::new();
    let any_off = AnyFlatRaw::serialize(&mut fbb, &value).unwrap();
    fbb.finish_minimal(any_off);
    let data = fbb.finished_data();
    if !expected.is_empty() {
        assert_eq!(expected, data);
    }
    println!("{}", data.chunks(8).map(|x| format!("{:?}", x)).join("\n"));
    let any = root::<AnyFlatRaw>(data).unwrap();
    let x = any.deserialize::<T>().unwrap();
    assert_eq!(x, value);
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Identity<T>(T);

#[test]
fn test() {
    run_test(
        10u8,
        &[
            7, 0, 0, 0, // root ptr
            0, 0, 0,    // padding
            10           // value
        ]);
    run_test(
        (0x01u8, 0x0203u16),
        &[
            12, 0, 0, 0, // root ptr
            8, 0, // vtable length
            8, 0, // inlined bytes
            7, 0, // first field offset
            4, 0, // second field offset
            8, 0, 0, 0, // vtable ptr
            3, 2, // second field
            0, // padding
            1 // first field
        ]);
    run_test(
        (0x0102u16, 0x03u8), &[
            12, 0, 0, 0,//root
            8, 0,//vtable length
            8, 0,//vtable inlined bytes
            6, 0,//vtable first field
            5, 0,//vtable second field
            8, 0, 0, 0,//vptr
            0,//padding
            3,//second field
            2, 1 //first field
        ]);
    run_test::<Option<u8>>(
        None,
        &[
            4, 0, 0, 0,//root
            0, 0, 0, 0 //ptr
        ]);
    run_test::<Option<u8>>(
        Some(55u8),
        &[
            4, 0, 0, 0,//root
            7, 0, 0, 0,//ptr
            0, 0, 0,//padding
            55//value
        ],
    );
    run_test::<Option<Option<u8>>>(
        None,
        &[4, 0, 0, 0, 0, 0, 0, 0]);
    run_test::<Option<Option<u8>>>(
        Some(None),
        &[
            4, 0, 0, 0,//root
            4, 0, 0, 0,//Some ptr
            0, 0, 0, 0,//None ptr
        ]);
    run_test::<Option<Option<u8>>>(Some(Some(55u8)), &[
        4, 0, 0, 0,//root
        4, 0, 0, 0,//Some ptr
        7, 0, 0, 0,//Some ptr
        0, 0, 0,//padding
        55,//value
    ]);
    run_test::<Option<Option<Option<u8>>>>(None, &[]);
    run_test::<Option<Option<Option<u8>>>>(Some(None), &[]);
    run_test::<Option<Option<Option<u8>>>>(Some(Some(None)), &[]);
    run_test::<Option<Option<Option<u8>>>>(Some(Some(Some(55u8))), &[]);
    run_test(
        Identity(0u8),
        &[
            8, 0, 0, 0, // root
            4, 0, // vtable length
            4, 0, // inline length
            4, 0, 0, 0,//vptr
        ]);
    run_test::<Identity<Option<u8>>>(
        Identity(None),
        &[
            8, 0, 0, 0, //root
            4, 0, //vtable length
            4, 0, //inline length
            4, 0, 0, 0 //vptr
        ]);
    run_test::<Identity<Option<u8>>>(Identity(Some(55u8)), &[
        12, 0, 0, 0,//root
        0, 0,//padding
        6, 0,
        11, 0,
        4, 0,
        6, 0, 0, 0,
        7, 0, 0, 0,
        0, 0, 0,
        55
    ]);
    run_test::<Identity<Option<Option<u8>>>>(Identity(None), &[
        8, 0, 0, 0, //root
        4, 0, //vtable length
        4, 0, //inline length
        4, 0, 0, 0 //vptr
    ]);
    run_test::<Identity<Option<Option<u8>>>>(Identity(Some(None)), &[
        12, 0, 0, 0,//root
        0, 0,
        6, 0,
        8, 0,
        4, 0,
        6, 0, 0, 0,
        4, 0, 0, 0,
        0, 0, 0, 0
    ]);
    run_test::<Identity<Option<Option<u8>>>>(Identity(Some(Some(55u8))), &[
        12, 0, 0, 0,//root
        0, 0,
        6, 0,
        8, 0,
        4, 0,
        6, 0, 0, 0,//vptr
        4, 0, 0, 0, //Some
        7, 0, 0, 0,//Some
        0, 0, 0, 55//value
    ]);
    run_test::<Identity<Option<Option<Option<u8>>>>>(Identity(None), &[]);
    run_test::<Identity<Option<Option<Option<u8>>>>>(Identity(Some(None)), &[]);
    run_test::<Identity<Option<Option<Option<u8>>>>>(Identity(Some(Some(None))), &[]);
    run_test::<Identity<Option<Option<Option<u8>>>>>(Identity(Some(Some(Some(55u8)))), &[]);
    run_test::<Vec<u8>>(
        vec![],
        &[
            4, 0, 0, 0,
            0, 0, 0, 0,
        ]);
    run_test(
        vec![1u8],
        &[
            4, 0, 0, 0,
            1, 0, 0, 0,
            1,
            0, 0, 0
        ]);
    run_test(
        vec![1u8, 2u8],
        &[
            4, 0, 0, 0,
            2, 0, 0, 0,
            1, 2,
            0, 0
        ]);
    run_test(
        vec![0x1234_5678_1234_5678_1234_5678_1234_5678u128],
        &[
            4, 0, 0, 0, 1, 0, 0, 0, 120, 86, 52, 18, 120, 86, 52, 18, 120, 86, 52, 18, 120, 86, 52, 18
        ]);
    run_test::<Vec<Option<u8>>>(
        vec![None],
        &[
            4, 0, 0, 0,
            1, 0, 0, 0,
            0, 0, 0, 0]);
    run_test::<Vec<Option<u8>>>(
        vec![None, None, None, None, None],
        &[
            4, 0, 0, 0,
            5, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0,
            0, 0, 0, 0]);
    run_test::<Vec<Option<u8>>>(
        vec![Some(55), Some(66)],
        &[
            4, 0, 0, 0,
            2, 0, 0, 0,
            11, 0, 0, 0,
            6, 0, 0, 0,
            0, 0,
            66,
            55]);
    run_test::<Vec<Option<Option<u8>>>>(
        vec![None, Some(None), Some(Some(55))],
        &[
            4, 0, 0, 0,
            3, 0, 0, 0,
            0, 0, 0, 0,
            16, 0, 0, 0,
            4, 0, 0, 0,
            7, 0, 0, 0,
            0, 0, 0,
            55,
            0, 0, 0, 0]);
    run_test::<Result<u8, u8>>(
        Ok(55),
        &[
            12, 0, 0, 0, //root
            8, 0,
            8, 0,
            6, 0, // tag offset
            5, 0, // value offset
            8, 0, 0, 0, //vptr
            0,
            55,//value
            0, 0//tag
        ]);
    run_test::<Result<u8, u8>>(
        Err(55),
        &[
            12, 0, 0, 0, //root
            8, 0,
            8, 0,
            6, 0, // tag offset
            5, 0, // value offset
            8, 0, 0, 0, //vptr
            0,
            55,//value
            1, 0//tag
        ]);
    run_test::<Result<u8, Result<u8, u8>>>(
        Err(Ok(55)),
        &[
            12, 0, 0, 0, // root
            8, 0,
            12, 0,
            10, 0, // variant offset
            4, 0, // value offset
            8, 0, 0, 0, //vptr
            16, 0, 0, 0, // Ok(55)
            0, 0,
            1, 0, //Err
            8, 0,
            8, 0,
            6, 0, //variant offset
            5, 0, //value offset
            8, 0, 0, 0,//vptr
            0,
            55, //55
            0, 0 //Ok
        ]);
    run_test::<Vec<Result<u8, u8>>>(
        vec![Ok(55), Err(66)],
        &[
            4, 0, 0, 0, 2, 0, 0, 0, 24, 0, 0, 0, 4, 0, 0, 0, 248, 255, 255, 255, 0, 66, 1, 0, 8, 0, 8, 0, 6, 0, 5, 0, 8, 0, 0, 0, 0, 55, 0, 0
        ]);
}