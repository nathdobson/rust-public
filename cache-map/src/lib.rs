#![allow(proc_macro_back_compat)]
#![allow(unreachable_code)]
#![deny(unused_must_use)]

#[macro_use]
extern crate rental;

use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
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

#[derive(Debug)]
pub struct InsertError;

impl Display for InsertError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "element already present in CacheMap")
    }
}

impl<K, V> CacheMap<K, V> {
    pub fn new() -> Self {
        CacheMap(CacheMapInner::new(Box::new(Arena::new()), |_| Mutex::new(HashMap::new())))
    }
}

impl<K: Hash + Eq, V> CacheMap<K, V> {
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
    pub fn get<'a, Q>(&'a self, k: &Q) -> Option<&'a V> where K: Borrow<Q>, Q: ?Sized + Hash + Eq {
        self.0.maybe_ref_rent(|map| Self::get_impl(map, k))
    }
    fn get_impl<'arena, Q>(map: &Mutex<HashMap<K, &'arena V>>, k: &Q) -> Option<&'arena V> where K: Borrow<Q>, Q: ?Sized + Hash + Eq {
        map.lock().get(k).map(|x| &**x)
    }
    pub fn get_or_init<'a, Q, F>(
        &'a self,
        k: &Q,
        f: F,
    ) -> &'a V
        where K: Borrow<Q>,
              Q: ?Sized + Eq + Hash + ToOwned<Owned=K>,
              F: FnOnce() -> V
    {
        self.0.ref_rent_all(|all| Self::get_or_init_impl(all.arena, all.map, k, f))
    }
    fn get_or_init_impl<'arena, Q, F>(
        arena: &'arena Arena<V>,
        map: &Mutex<HashMap<K, &'arena V>>,
        k: &Q,
        f: F,
    ) -> &'arena V
        where K: Borrow<Q>,
              Q: ?Sized + Hash + Eq + ToOwned<Owned=K>,
              F: FnOnce() -> V,
    {
        let mut lock = map.lock();
        if let Some(value) = lock.get(k) {
            *value
        } else {
            lock.entry(k.to_owned()).insert_entry(arena.alloc(f())).get()
        }
    }
}

#[test]
fn test() {
    let map = CacheMap::new();
    map.try_insert(1, 2).unwrap();
    let x = map.get(&1).unwrap();
    map.try_insert(2, 4).unwrap();
    let y = map.get(&2).unwrap();
    assert_eq!(x, &2);
    assert_eq!(y, &4);
}