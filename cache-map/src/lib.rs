#![feature(entry_insert)]
#![allow(proc_macro_back_compat)]
#![allow(unreachable_code)]
#![deny(unused_must_use)]

use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use parking_lot::Mutex;
use colosseum::sync::Arena;
use ouroboros::self_referencing;
use ouroboros_impl_cache_map_inner::BorrowedFields;

#[self_referencing]
pub struct CacheMapInner<K: 'static, V: 'static> {
    arena: Arena<V>,
    #[borrows(arena)]
    #[not_covariant]
    map: Mutex<HashMap<K, &'this V>>,
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
        CacheMap(CacheMapInner::new(Arena::new(), |_| Mutex::new(HashMap::new())))
    }
}

impl<K: Hash + Eq, V> CacheMap<K, V> {
    pub fn try_insert(&mut self, k: K, v: V) -> Result<(), InsertError> {
        self.0.with_mut(|all| Self::try_insert_impl(all.arena, all.map, k, v))
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
        fn imp<'outer, 'q, K, V, Q>(
            k: &'q Q
        ) ->
            impl 'q + for<'arena> FnOnce(
                BorrowedFields<'outer, 'arena, K, V>
            ) -> Option<&'outer V>
            where K: Eq + Hash + Borrow<Q>,
                  Q: ?Sized + Eq + Hash {
            |all| {
                all.map.lock().get(k).map(|x| &**x)
            }
        }
        self.0.with(imp(k))
    }
    pub fn get_or_init<'a, 'q, Q, F: 'q>(
        &'a self,
        k: &'q Q,
        f: F,
    ) -> &'a V
        where K: Borrow<Q>,
              Q: ?Sized + Eq + Hash + ToOwned<Owned=K>,
              F: 'q + FnOnce() -> V
    {
        fn imp<'outer, 'q, K, V, Q, F: 'q>(
            k: &'q Q,
            f: F,
        ) ->
            impl 'q + for<'arena> FnOnce(
                BorrowedFields<'outer, 'arena, K, V>
            ) -> &'outer V
            where K: Eq + Hash + Borrow<Q>,
                  Q: ?Sized + Eq + Hash + ToOwned<Owned=K>,
                  F: FnOnce() -> V {
            |all| {
                let mut lock = all.map.lock();
                if let Some(value) = lock.get(k) {
                    *value
                } else {
                    lock.entry(k.to_owned()).insert_entry(all.arena.alloc(f())).get()
                }
            }
        }
        self.0.with(imp(k, f))
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