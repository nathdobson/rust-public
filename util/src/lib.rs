#![feature(unsize, coerce_unsized, rustc_private, negative_impls, never_type, write_all_vectored, vec_into_raw_parts, raw, can_vector, bound_cloned, backtrace)]

use std::mem;
use std::collections::BTreeMap;
use std::sync::Arc;

pub mod bag;
pub mod listen;
pub mod union;
pub mod shared;
pub mod socket;
pub mod dirty;
pub mod rng;
pub mod grid;
pub mod range;
pub mod rect;
//pub mod itertools;
pub mod io;
pub mod rangemap;
pub mod profile;
pub mod completable;
pub mod lossy;
pub mod watch;
pub mod version;
pub mod pmpsc;
pub mod cancel;
pub mod shutdown;
pub mod expect;

pub fn btree_set_keys<'a, Q: 'a + ?Sized, V>(
    map: &'a mut BTreeMap<Q::Owned, V>,
    keys: impl IntoIterator<Item=&'a Q>,
    mut new: impl FnMut(&Q) -> V,
    mut old: impl FnMut(Q::Owned, V))
    where Q: ToOwned,
          Q: Ord,
          Q::Owned: Ord
{
    let new_map: BTreeMap<Q::Owned, V> =
        keys.into_iter()
            .map(|k| {
                map.remove_entry(k)
                    .unwrap_or_else(|| (k.to_owned(), new(k)))
            })
            .collect();
    for (k, v) in mem::replace(map, new_map).into_iter() {
        old(k, v);
    }
}

#[test]
fn btreemap_set_keys_test() {
    use std::collections::BTreeSet;
    let mut map: BTreeMap<String, usize> = [("a".to_owned(), 1), ("b".to_owned(), 2)].iter().cloned().collect();
    let set: BTreeSet<&'static str> = ["b", "c"].iter().cloned().collect();
    btree_set_keys(&mut map, set.iter().map(|x| *x), |k| {
        assert_eq!(k, "c");
        3
    }, |k, v| {
        assert_eq!(k, "a");
        assert_eq!(v, 1);
    });
    assert_eq!(map, [("b".to_owned(), 2), ("c".to_owned(), 3)].iter().cloned().collect::<BTreeMap<_, _>>())
}

pub type Name = Arc<String>;