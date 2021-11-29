#![feature(once_cell)]
#![deny(unused_must_use)]
#![allow(unused_mut)]

use registry::{Registry, BuilderFrom, Builder};
use registry::registry;
use std::collections::{HashMap, HashSet};
use std::lazy::SyncOnceCell;

struct TestRegistry(HashMap<&'static str, &'static str>);

struct TestRegistryLazy(Vec<&'static TestEntryLazy>);

struct TestEntry {
    key: &'static str,
    value: &'static str,
}

struct TestEntryLazy {
    cell: SyncOnceCell<usize>,
}

static TEST_REGISTRY: Registry<TestRegistry> = Registry::new();

static TEST_REGISTRY_LAZY: Registry<TestRegistryLazy> = Registry::new();

impl Builder for TestRegistry {
    type Output = HashMap<&'static str, &'static str>;
    fn new() -> Self { TestRegistry(HashMap::new()) }
    fn build(self) -> HashMap<&'static str, &'static str> { self.0 }
}

impl Builder for TestRegistryLazy {
    type Output = ();
    fn new() -> Self { TestRegistryLazy(vec![]) }
    fn build(mut self) {
        for (i, x) in self.0.into_iter().enumerate() {
            x.cell.set(i).unwrap();
        }
    }
}

impl BuilderFrom<TestEntry> for TestRegistry {
    fn insert(&mut self, element: TestEntry) {
        self.0.insert(element.key, element.value);
    }
}

impl BuilderFrom<&TestEntry> for TestRegistry {
    fn insert(&mut self, element: &TestEntry) {
        self.0.insert(element.key, element.value);
    }
}

impl BuilderFrom<&'static TestEntryLazy> for TestRegistryLazy {
    fn insert(&mut self, element: &'static TestEntryLazy) {
        self.0.push(element)
    }
}

impl TestEntryLazy {
    fn new() -> Self {
        TestEntryLazy { cell: SyncOnceCell::new() }
    }
}

registry! {
    require foo;
    value TEST_REGISTRY => TestEntry { key: "a", value: "a" };
    static TEST_REGISTRY => TEST_ENTRY: TestEntry = TestEntry { key: "c", value: "c"};
    lazy TEST_REGISTRY_LAZY => TEST_ENTRY_LAZY1: TestEntryLazy = TestEntryLazy::new();
    lazy TEST_REGISTRY_LAZY => TEST_ENTRY_LAZY2: TestEntryLazy = TestEntryLazy::new();
}

mod foo {
    use registry::registry;
    registry! {
        value crate::TEST_REGISTRY => crate::TestEntry { key: "b", value: "b" };
    }
}

#[test]
fn test() {
    REGISTRY.build();
    assert_eq!(*TEST_REGISTRY, vec![("a", "a"), ("b", "b"), ("c", "c")].into_iter().collect());
    assert_eq!(vec![0, 1].into_iter().collect::<HashSet<_>>(),
               vec![*TEST_ENTRY_LAZY1.cell.get().unwrap(),
                    *TEST_ENTRY_LAZY2.cell.get().unwrap()]
                   .into_iter().collect());
}