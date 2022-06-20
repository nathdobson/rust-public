use std::any::{Any, TypeId};
use std::borrow::Borrow;
use std::default::default;
use std::hash::Hash;
use std::sync::Arc;
use cache_map::CacheMap;
use crate::SafeOnceCell;

pub struct SafeOnceCellMap<K: 'static, V: 'static> {
    map: CacheMap<K, Arc<SafeOnceCell<V>>>,
}

pub struct SafeTypeMap {
    map: SafeOnceCellMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl<K: Eq + Hash, V> SafeOnceCellMap<K, V> {
    pub fn get_or_init<'a, Q, F>(&'a self, key: &Q, f: F) -> &'a V
        where
            Q: ?Sized + ToOwned<Owned = K>,
            K: Borrow<Q>,
            F: FnOnce() -> V,
            Q: Eq + Hash,
    {
        self.map.get_or_init(key, default).get_or_init(f)
    }
}

impl<K, V> SafeOnceCellMap<K, V> {
    pub fn new() -> Self {
        SafeOnceCellMap {
            map: CacheMap::new(),
        }
    }
}

impl SafeTypeMap {
    pub fn new() -> Self {
        SafeTypeMap {
            map: SafeOnceCellMap::new(),
        }
    }
    pub fn get_or_init<'a, F, T: 'static + Send + Sync>(&'a self, f: F) -> &'a T
        where
            F: FnOnce() -> T,
    {
        let type_id = TypeId::of::<T>();
        let result: &'a Box<dyn Any + Send + Sync> =
            self.map.get_or_init(&type_id, || Box::new(f()));
        result.downcast_ref::<T>().unwrap()
    }
}

impl Default for SafeTypeMap {
    fn default() -> Self { SafeTypeMap::new() }
}