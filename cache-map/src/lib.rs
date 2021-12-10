#![allow(proc_macro_back_compat)]

#[macro_use]
extern crate rental;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use parking_lot::Mutex;
use rental::rental;
use rented::CacheMapInner;
use colosseum::sync::Arena;

rental! {
    mod rented {
        use parking_lot::Mutex;
        use std::collections::HashMap;
        use colosseum::sync::Arena;
        #[rental]
        pub struct CacheMapInner<K:'static , V: 'static> {
            arena: Box<Arena<V>>,
            map: Mutex<HashMap<K, &'arena V>>,
        }
    }
}

pub struct CacheMap<K: 'static, V: 'static>(CacheMapInner<K, V>);

pub struct InsertError;

impl<K: Hash + Eq, V> CacheMap<K, V> {
    pub fn new() -> Self {
        CacheMap(CacheMapInner::new(Box::new(Arena::new()), |_| Mutex::new(HashMap::new())))
    }
    pub fn try_insert(&self, k: K, v: V) -> Result<(), InsertError> {
        self.0.rent_all(|all| Self::try_insert_impl(all.arena, all.map, k, v))
    }
    fn try_insert_impl<'arena>(arena: &'arena Arena<V>, map: &Mutex<HashMap<K, &'arena V>>, k: K, v: V) -> Result<(), InsertError> {
        match map.lock().entry(k) {
            Entry::Occupied(_) => Err(InsertError),
            Entry::Vacant(vacant) => {
                let v = arena.alloc(v);
                vacant.insert(v);
                Ok(())
            }
        }
    }
    pub fn get<'a>(&'a self, k: &K) -> Option<&'a V> {
        self.0.maybe_ref_rent(|map| Self::get_impl(map, k))
    }
    fn get_impl<'arena>(map: &Mutex<HashMap<K, &'arena V>>, k: &K) -> Option<&'arena V> {
        map.lock().get(k).map(|x| &**x)
    }
}

#[test]
fn test() {
    let map = CacheMap::new();
    map.insert(1, 2);
    let x = map.get(&1).unwrap();
    map.insert(2, 4);
    let y = map.get(&2).unwrap();
    assert_eq!(x, &2);
    assert_eq!(y, &4);
}