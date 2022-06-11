#![feature(specialization, never_type)]
#![allow(
    incomplete_features,
    unused_variables,
    dead_code,
    unused_imports,
    unused_macros,
    unused_mut
)]
#![deny(unused_must_use)]
#![feature(once_cell)]

mod common;

use std::ops::Deref;

use common::any_string::AnyString;
use common::custom;
use serde::{Deserialize, Serialize};
use typetag_static::{binary, json, BoxAnySerde};

use crate::common::custom::{Custom, Expected};

#[test]
fn test_binary_any() {
    //common::REGISTRY.build();
    let input = AnyString("abcd".to_string());
    let encoded = binary::serialize(&(Box::new(input.clone()) as BoxAnySerde)).unwrap();
    assert_eq!(
        vec![
            180, 166, 84, 202, 194, 153, 125, 1, 216, 27, 194, 70, 212, 225, 67, 100, 12, 0, 0, 0,
            0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 97, 98, 99, 100
        ],
        encoded
    );
    assert_eq!(
        &input,
        binary::deserialize::<BoxAnySerde>(&encoded)
            .unwrap()
            .deref()
            .downcast_ref::<AnyString>()
            .unwrap()
    );
}

#[test]
fn test_binary_unknown() {
    // common::REGISTRY.build();
    let input = vec![
        1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, 8, 0, 0, 0, 0, 0, 0, 0, 11, 12, 13, 14, 15,
        16, 17, 18,
    ];
    let decoded = binary::deserialize::<BoxAnySerde>(&input).unwrap();
    let encoded = binary::serialize(&decoded).unwrap();
    assert_eq!(input, encoded);
}

#[test]
fn test_binary_string() {
    // common::REGISTRY.build();
    let input = "abcd".to_string();
    let encoded = binary::serialize(&(Box::new(input.clone()) as BoxAnySerde)).unwrap();
    assert_eq!(
        vec![
            247, 72, 104, 66, 51, 227, 114, 137, 133, 127, 33, 123, 240, 188, 154, 122, 12, 0, 0,
            0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 97, 98, 99, 100
        ],
        encoded
    );
    assert_eq!(
        &input,
        binary::deserialize::<BoxAnySerde>(&encoded)
            .unwrap()
            .deref()
            .downcast_ref::<String>()
            .unwrap()
    );
}

#[test]
fn test_json_any() {
    // common::REGISTRY.build();
    let input = AnyString("abcd".to_string());
    let encoded = json::serialize(&(Box::new(input.clone()) as BoxAnySerde)).unwrap();
    assert_eq!(r#"{"serde_any::tests::common::AnyString":"abcd"}"#, encoded);
    assert_eq!(
        &input,
        json::deserialize::<BoxAnySerde>(encoded.as_bytes())
            .unwrap()
            .deref()
            .downcast_ref::<AnyString>()
            .unwrap()
    );
}

#[test]
fn test_json_string() {
    // common::REGISTRY.build();
    let input = "abcd".to_string();
    let encoded = json::serialize(&(Box::new(input.clone()) as BoxAnySerde)).unwrap();
    assert_eq!(r#"{"std::string::String":"abcd"}"#, encoded);
    assert_eq!(
        &input,
        json::deserialize::<BoxAnySerde>(encoded.as_bytes())
            .unwrap()
            .deref()
            .downcast_ref::<String>()
            .unwrap()
    );
}

#[test]
fn test_json_unknown() {
    // common::REGISTRY.build();
    let input = r#"{"????":"abcd"}"#;
    let decoded = json::deserialize::<BoxAnySerde>(input.as_bytes()).unwrap();
    let encoded = json::serialize(&decoded).unwrap();
    assert_eq!(input, encoded);
}

#[test]
fn test_custom_serializer() {
    // common::REGISTRY.build();
    assert_eq!(
        Expected,
        (Box::new(AnyString("abcd".to_string())) as BoxAnySerde)
            .serialize(Custom)
            .unwrap()
    );
}

#[test]
fn test_custom_deserializer() {
    // common::REGISTRY.build();
    assert_eq!(
        &AnyString("abcd".to_string()),
        BoxAnySerde::deserialize(Custom)
            .unwrap()
            .deref()
            .downcast_ref::<AnyString>()
            .unwrap()
    );
}
