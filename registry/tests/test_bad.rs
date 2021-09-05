use registry::{Registry, Builder, BuilderFrom};
use registry::registry;

struct TestEntry;

struct TestRegistry;

impl BuilderFrom<TestEntry> for TestRegistry {
    fn insert(&mut self, element: TestEntry) {}
}

impl Builder for TestRegistry {
    type Output = Self;
    fn new() -> Self { TestRegistry }
    fn build(self) -> Self::Output { self }
}

static TEST_REGISTRY: Registry<TestRegistry> = Registry::new();

registry! {
    value TEST_REGISTRY => TestEntry;
}

mod foo {
    use registry::registry;
    registry! {
        value crate::TEST_REGISTRY => crate::TestEntry;
    }
}

#[test]
#[should_panic(expected = "Registry not built for test_bad::foo")]
fn test() {
    REGISTRY.build();
    &*TEST_REGISTRY;
}