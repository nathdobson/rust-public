#![deny(unused_must_use)]
#![allow(unused_imports)]
use registry::registry;
use flatbuffers_serde::tag::TYPE_TAGS;

registry! {
    require flatbuffers_serde;
}


#[test]
fn test_type_tag() {
    REGISTRY.build();
    println!("{:#?}", TYPE_TAGS);
}