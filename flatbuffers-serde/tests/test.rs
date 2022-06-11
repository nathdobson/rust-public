#![deny(unused_must_use)]
#![allow(unused_imports)]
use flatbuffers_serde::tag::TYPE_TAGS;
use registry::registry;

registry! {
    require flatbuffers_serde;
}

#[test]
fn test_type_tag() {
    REGISTRY.build();
    println!("{:#?}", TYPE_TAGS);
}
