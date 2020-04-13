use std::any::Any;
use std::fmt;
use std::hash::{Hasher, Hash};
use std::cmp::Ordering;

pub trait ObjectInner: Any + fmt::Debug + 'static + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn eq_any(&self, other: &dyn Any) -> bool;
    fn hash_any(&self, state: &mut dyn Hasher);
    fn partial_cmp_any(&self, other: &dyn Any) -> Option<Ordering>;
    fn cmp_any(&self, other: &dyn Any) -> Ordering;
    fn clone_inner(&self) -> Box<dyn ObjectInner>;
}

impl<T> ObjectInner for T where T: PartialEq + fmt::Debug + Hash + 'static + PartialOrd + Ord + Eq + Clone + Send + Sync {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn eq_any(&self, other: &dyn Any) -> bool {
        other.downcast_ref().map_or(false, |other| self == other)
    }
    fn hash_any(&self, mut state: &mut dyn Hasher) {
        self.type_id().hash(&mut state);
        self.hash(&mut state);
    }

    fn partial_cmp_any(&self, other: &dyn Any) -> Option<Ordering> {
        other.downcast_ref().map_or(Some(self.type_id().cmp(&other.type_id())), |other| self.partial_cmp(&other))
    }

    fn cmp_any(&self, other: &dyn Any) -> Ordering {
        other.downcast_ref().map_or(self.type_id().cmp(&other.type_id()), |other| self.cmp(&other))
    }

    fn clone_inner(&self) -> Box<dyn ObjectInner> {
        Box::new(self.clone())
    }
}

#[derive(Debug)]
pub struct Object(Box<dyn ObjectInner>);

impl Object {
    pub fn new<T: ObjectInner>(x: T) -> Object {
        Object(Box::new(x))
    }
    pub fn as_any(&self) -> &dyn Any {
        self.0.as_any()
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_any(other.0.as_any())
    }
}

impl Eq for Object {}

impl PartialOrd for Object {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp_any(other.0.as_any())
    }
}

impl Ord for Object {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp_any(other.0.as_any())
    }
}

impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash_any(state)
    }
}

impl Clone for Object {
    fn clone(&self) -> Self {
        Object(self.0.clone_inner())
    }
}

#[test]
fn test_anyeq() {
    use std::collections::HashMap;
    let object1 = Object::new(1);
    let object2 = Object::new(1);
    let object3 = Object::new(2);
    let object4 = Object::new("a");
    assert_eq!(object1, object2);
    assert!(object1 != object3);
    assert!(object1 != object4);
    let mut map = HashMap::new();
    map.insert(object1.clone(), "x");
    map.insert(object2.clone(), "y");
    map.insert(object3.clone(), "z");
    map.insert(object4.clone(), "w");
    assert_eq!(map.get(&object1), Some(&"y"));
    assert_eq!(map.get(&object2), Some(&"y"));
    assert_eq!(map.get(&object3), Some(&"z"));
    assert_eq!(map.get(&object4), Some(&"w"));
}