use registry::Registry;
use registry::registry;

struct TestEntry;

static TEST_REGISTRY
: Registry<TestEntry, ()>
= Registry::new(|_| ());

registry! {
    register(TEST_REGISTRY) { TestEntry }
}

mod foo {
    use registry::registry;
    registry! {
        register(crate::TEST_REGISTRY) { crate::TestEntry }
    }
}

#[test]
#[should_panic(expected = "Registry not built for test_bad::foo")]
fn test() {
    REGISTRY.build();
    *TEST_REGISTRY;
}