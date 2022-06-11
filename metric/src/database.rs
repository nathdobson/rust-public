use std::borrow::Cow;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use rusqlite::Connection;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub rows: Vec<SnapshotRow>,
}

#[derive(Clone, Debug)]
pub struct SnapshotRow {
    pub keys: Vec<SnapshotCell>,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct SnapshotCell {
    pub name: &'static str,
    pub value: SnapshotValue,
}

#[derive(Clone, Debug)]
pub enum SnapshotValue {
    String(&'static str),
    Integer(i64),
    Float(f64),
}

impl From<&'static str> for SnapshotValue {
    fn from(x: &'static str) -> Self { SnapshotValue::String(x) }
}

impl From<i64> for SnapshotValue {
    fn from(x: i64) -> Self { SnapshotValue::Integer(x) }
}

impl From<u64> for SnapshotValue {
    fn from(x: u64) -> Self { SnapshotValue::Integer(x as i64) }
}

impl From<i32> for SnapshotValue {
    fn from(x: i32) -> Self { SnapshotValue::Integer(x as i64) }
}

impl From<u32> for SnapshotValue {
    fn from(x: u32) -> Self { SnapshotValue::Integer(x as i64) }
}

impl From<f64> for SnapshotValue {
    fn from(x: f64) -> Self { SnapshotValue::Float(x) }
}

impl FromIterator<SnapshotRow> for Snapshot {
    fn from_iter<T: IntoIterator<Item = SnapshotRow>>(iter: T) -> Self {
        Snapshot {
            rows: iter.into_iter().collect(),
        }
    }
}

// impl SnapshotCell {
//     pub fn new<V>(column: &'static str, value: V) -> Self where SnapshotValue: From<V> {
//         SnapshotCell { column, value: SnapshotValue::from(value) }
//     }
// }

// struct Inner {
//     callbacks: Vec<Callback>,
// }
//
// #[derive(Clone)]
// pub struct Database(Arc<Mutex<Inner>>);
//
// lazy_static! {
//     static ref DATABASE: Database = Database::new();
// }
//
// type Callback = Box<dyn FnMut(&mut Connection) + Send>;
//
// impl Database {
//     fn new() -> Database {
//         Database(Arc::new(Mutex::new(Inner { callbacks: vec![] })))
//     }
//     pub fn add_connection(mut conn: Connection) {
//         thread::executor(move || {
//             loop {
//                 thread::sleep(Duration::from_millis(1000));
//                 for cb in DATABASE.0.lock().callbacks.iter_mut() {
//                     cb(&mut conn)
//                 }
//             }
//         });
//     }
//     pub fn add_callback(callback: Callback) {
//         DATABASE.0.lock().callbacks.push(callback);
//     }
// }
