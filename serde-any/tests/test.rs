#![feature(specialization, never_type, const_fn_fn_ptr_basics)]
#![allow(incomplete_features, unused_variables, dead_code, unused_imports, unused_macros, unused_mut)]
#![deny(unused_must_use)]

mod common;

use common::any_string::AnyString;
use common::custom;
use serde_any::binary;
use std::ops::Deref;
use serde_any::any::AnySerde;


#[test]
fn test_binary_any() {
    let input = AnyString("abcd".to_string());
    let encoded = binary::serialize(&AnySerde::new(input.clone())).unwrap();
    assert_eq!(vec![
        180, 166, 84, 202, 194, 153, 125, 1,
        216, 27, 194, 70, 212, 225, 67, 100,
        182, 212, 71, 188, 231, 219, 87, 45,
        101, 254, 115, 158, 123, 92, 252, 171,
        12, 0, 0, 0, 0, 0, 0, 0,
        4, 0, 0, 0, 0, 0, 0, 0,
        97, 98, 99, 100], encoded);
    assert_eq!(&input, binary::deserialize::<AnySerde>(
        &encoded).unwrap().deref().as_any().downcast_ref::<AnyString>().unwrap());
}
