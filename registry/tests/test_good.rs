use registry::Registry;
use registry::registry;
use std::collections::HashMap;


struct TestEntry {
    key: &'static str,
    value: &'static str,
}

static TEST_REGISTRY
: Registry<TestEntry, HashMap<&'static str, &'static str>>
= Registry::new(|entries| entries.into_iter().map(|x| (x.key, x.value)).collect());

registry! {
    require foo;
    register(TEST_REGISTRY) { TestEntry { key: "a", value: "a" } }
}

mod foo {
    use registry::registry;
    registry! {
        register(crate::TEST_REGISTRY) { crate::TestEntry { key: "b", value: "b" } }
    }
}

#[test]
fn test() {
    REGISTRY.build();
    assert_eq!(*TEST_REGISTRY, vec![("a", "a"), ("b", "b")].into_iter().collect());
}