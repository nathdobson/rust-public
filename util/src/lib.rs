#![feature(
    unsize,
    coerce_unsized,
    rustc_private,
    negative_impls,
    never_type,
    write_all_vectored,
    vec_into_raw_parts,
    raw,
    can_vector,
    bound_cloned,
    backtrace,
    type_name_of_val
)]
#![allow(unused_imports)]
#![feature(arbitrary_self_types, unboxed_closures)]
#![feature(arc_new_cyclic)]
#![feature(unsafe_cell_raw_get)]
#![feature(raw_ref_op)]
#![feature(box_syntax)]
#![feature(associated_type_defaults)]
#![feature(specialization)]
#![feature(slice_ptr_len)]
#![feature(slice_index_methods)]
#![feature(maybe_uninit_extra)]
#![feature(fn_traits)]
#![feature(allocator_api)]
#![allow(incomplete_features)]
#![allow(unused_variables)]

use std::collections::BTreeMap;
use std::marker::{PhantomData, Unsize};
use std::mem;
use std::ops::Deref;
use std::sync::Arc;

pub mod any;
pub mod atomic_refcell;
pub mod completable;
pub mod dirty;
pub mod expect;
pub mod fun;
pub mod grid;
pub mod io;
pub mod itertools;
pub mod listen;
pub mod lossy;
pub mod mutrc;
pub mod pmpsc;
pub mod profile;
pub mod range;
pub mod rangemap;
pub mod rect;
pub mod rng;
pub mod shared;
pub mod shared_cell;
pub mod slice;
pub mod socket;
pub mod sums;
pub mod take_cell;
pub mod take_vec;
pub mod time;
pub mod tree;
pub mod union;
pub mod version;
pub mod watch;
pub mod weak_vec;
pub mod web;

pub fn btree_set_keys<'a, Q: 'a + ?Sized, V>(
    map: &'a mut BTreeMap<Q::Owned, V>,
    keys: impl IntoIterator<Item = &'a Q>,
    mut new: impl FnMut(&Q) -> V,
    mut old: impl FnMut(Q::Owned, V),
) where
    Q: ToOwned,
    Q: Ord,
    Q::Owned: Ord,
{
    let new_map: BTreeMap<Q::Owned, V> = keys
        .into_iter()
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
    let mut map: BTreeMap<String, usize> = [("a".to_owned(), 1), ("b".to_owned(), 2)]
        .iter()
        .cloned()
        .collect();
    let set: BTreeSet<&'static str> = ["b", "c"].iter().cloned().collect();
    btree_set_keys(
        &mut map,
        set.iter().map(|x| *x),
        |k| {
            assert_eq!(k, "c");
            3
        },
        |k, v| {
            assert_eq!(k, "a");
            assert_eq!(v, 1);
        },
    );
    assert_eq!(
        map,
        [("b".to_owned(), 2), ("c".to_owned(), 3)]
            .iter()
            .cloned()
            .collect::<BTreeMap<_, _>>()
    )
}

pub type Name = Arc<String>;
