use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::mem;
use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::Connection;

use crate::database::{Snapshot, SnapshotRow};
use crate::keys::Keys;
use crate::values::Values;

pub struct Metric<K: Keys, V: Values + Clone> {
    stats: V,
    inner: Mutex<MetricInner<K, V>>,
}

pub struct LocalMetric<K: Keys, V: Values + Clone> {
    stats: V,
    cell: Arc<Mutex<HashMap<K, V::Set>>>,
}

struct MetricEntry<K: Keys, V: Values + Clone> {
    cell: Arc<Mutex<HashMap<K, V::Set>>>,
    cache: HashMap<K, V::Set>,
}

struct MetricInner<K: Keys, V: Values + Clone> {
    entries: Vec<MetricEntry<K, V>>,
    total: HashMap<K, V::Set>,
}

impl<K: Keys, V: Values + Clone> Metric<K, V> {
    pub fn new(stats: V) -> Self {
        Metric {
            stats,
            inner: Mutex::new(MetricInner {
                entries: vec![],
                total: HashMap::new(),
            }),
        }
    }
    pub fn local(&self) -> LocalMetric<K, V> {
        let mut lock = self.inner.lock();
        let cell = Arc::new(Mutex::new(HashMap::new()));
        lock.entries.push(MetricEntry {
            cell: cell.clone(),
            cache: HashMap::new(),
        });
        LocalMetric {
            stats: self.stats.clone(),
            cell,
        }
    }
    pub fn get(&self, result: &mut HashMap<K, V::Set>) {
        let mut this = self.inner.lock();
        let this = &mut *this;
        for entry in this.entries.iter_mut() {
            for value in entry.cache.values_mut() {
                self.stats.clear(value)
            }
            let mut cell = entry.cell.lock();
            mem::swap(&mut entry.cache, &mut *cell);
            mem::drop(cell);
            for (keys, value) in entry.cache.iter() {
                self.stats
                    .add_set(&mut this.total.entry(keys.clone()).or_default(), value);
            }
        }
        this.entries
            .retain(|entry| Arc::strong_count(&entry.cell) > 1);
        result.clone_from(&this.total);
    }
}

impl<K: Keys, V: Values + Clone> LocalMetric<K, V> {
    pub fn add(&self, key: K, p: V::Point) {
        self.stats
            .add_point(&mut *self.cell.lock().entry(key).or_default(), &p);
    }
}

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Sized + 'static> AsAny for T {
    fn as_any(&self) -> &dyn Any { self }
}

pub trait IsMetric: Send + Sync + AsAny {
    fn snapshot(&self) -> Snapshot;
}

impl<K: Keys, V: Values + Clone + Send + Sync> IsMetric for Metric<K, V> {
    fn snapshot(&self) -> Snapshot {
        let mut map = HashMap::new();
        self.get(&mut map);
        Snapshot {
            rows: map
                .into_iter()
                .map(|(ks, v)| SnapshotRow {
                    keys: ks.into_cells(),
                    value: vec![],
                })
                .collect(),
        }
    }
}
