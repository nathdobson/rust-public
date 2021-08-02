use std::hash::Hash;
use crate::values::Values;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::fmt::Debug;
use crate::database::{Snapshot, SnapshotCell, SnapshotValue};
use serde::{Serializer, Serialize};
use serde::ser::{SerializeSeq, SerializeMap, SerializeStructVariant, Error, SerializeTupleVariant, SerializeTuple, SerializeStruct, SerializeTupleStruct};


//
// #[derive(Debug)]
// pub struct Table<K: TableKey, V: Stats> {
//     names: Vec<&'static str>,
//     stats: V,
//     phantom: PhantomData<K>,
// }
//
// #[derive(Debug)]
// pub struct TablePoint<K: TableKey, V: Stats> {
//     pub key: K,
//     pub value: V::Point,
// }
//
// #[derive(Debug)]
// pub struct TableSet<K: TableKey, V: Stats> {
//     map: HashMap<K, V::Set>,
// }
//
//
// impl<K: TableKey, V: Stats> Default for TableSet<K, V> {
//     fn default() -> Self {
//         TableSet { map: Default::default() }
//     }
// }
//
// impl<K: TableKey, V: Stats> Table<K, V> {
//     pub fn new(names: Vec<&'static str>, stats: V) -> Self {
//         Table { names, stats, phantom: PhantomData }
//     }
//     // pub fn snapshot(&self, set:&TableSet<K,V>)->impl Iterator<Item=()>{
//     //     set.map.iter().map(|x|{})
//     //     // fn snapshot(&self, set: &TableSet<K, V>) -> Snapshot {
//     //     //     set.map.iter().flat_map(|(k, v)| {
//     //     //         let snapshot = self.stats.snapshot(v);
//     //     //         let prefix: Vec<SnapshotCell> = self.names.iter().zip(k.clone().into_values().into_iter()).map(|(column, value)| {
//     //     //             SnapshotCell { column, value }
//     //     //         }).collect();
//     //     //         snapshot.rows.into_iter().map(move |row| {
//     //     //             prefix.iter().cloned().chain(row.cells.iter().cloned()).collect()
//     //     //         })
//     //     //     }).collect()
//     //     // }
//     // }
// }
//
// impl<K: TableKey, V: Stats> Clone for TableSet<K, V> {
//     fn clone(&self) -> Self { TableSet { map: self.map.clone() } }
//     fn clone_from(&mut self, source: &Self) { self.map.clone_from(&source.map) }
// }
//
// impl<K: TableKey, V: Stats> Stats for Table<K, V> {
//     type Set = TableSet<K, V>;
//     type Point = TablePoint<K, V>;
//
//     fn add_point(&self, set: &mut TableSet<K, V>, point: &TablePoint<K, V>) {
//         self.stats.add_point(set.map.entry(point.key.clone()).or_default(), &point.value);
//     }
//
//     fn add_set(&self, set: &mut TableSet<K, V>, other: &TableSet<K, V>) {
//         for (k, v) in other.map.iter() {
//             self.stats.add_set(set.map.entry(k.clone()).or_default(), v);
//         }
//     }
//
//     fn clear(&self, set: &mut TableSet<K, V>) {
//         for (k, v) in set.map.iter_mut() {
//             self.stats.clear(v);
//         }
//     }
//
// }

