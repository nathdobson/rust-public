use std::any::Any;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use by_address::ByAddress;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use rusqlite::Connection;

use crate::keys::Keys;
use crate::metric::{IsMetric, LocalMetric, Metric};
use crate::values::Values;

#[derive(Clone, Copy, Eq, Ord, Hash, PartialEq, PartialOrd, Debug)]
pub struct MetricKey(ByAddress<&'static str>);

impl MetricKey {
    pub const fn new(name: &'static str) -> Self { MetricKey(ByAddress(name)) }
}

struct Inner {
    metrics: HashMap<MetricKey, Arc<dyn IsMetric>>,
}

#[derive(Clone)]
pub(crate) struct MetricSet(Arc<Mutex<Inner>>);

lazy_static! {
    static ref GLOBAL_SET: MetricSet = MetricSet::new();
}

thread_local! {
    static LOCAL_SET: RefCell<Option<MetricSet>> = RefCell::new(None);
}

impl MetricSet {
    pub fn new() -> Self {
        MetricSet(Arc::new(Mutex::new(Inner {
            metrics: HashMap::new(),
        })))
    }
    pub fn global() -> Self {
        LOCAL_SET
            .with(|local_set| local_set.borrow().clone())
            .unwrap_or(GLOBAL_SET.clone())
    }
    pub fn set_for_thread(&self) {
        LOCAL_SET.with(|local_set| *local_set.borrow_mut() = Some(self.clone()))
    }
    pub fn get_or_insert<F, T>(&self, key: MetricKey, default: F) -> Arc<dyn IsMetric>
    where
        F: FnOnce() -> Arc<T>,
        T: IsMetric + 'static,
    {
        self.0
            .lock()
            .metrics
            .entry(key)
            .or_insert_with(|| default())
            .clone()
    }
    pub fn get_local<K: Keys, V: Values + Clone>(
        &self,
        key: MetricKey,
        default: &V,
    ) -> LocalMetric<K, V> {
        self.get_or_insert(key, || Arc::new(Metric::<K, V>::new(default.clone())))
            .as_any()
            .downcast_ref::<Metric<K, V>>()
            .unwrap()
            .local()
    }
    pub fn get_keys(&self) -> Vec<MetricKey> { self.0.lock().metrics.keys().cloned().collect() }
    pub fn get(&self, key: MetricKey) -> Option<Arc<dyn IsMetric>> {
        self.0.lock().metrics.get(&key).cloned()
    }
}
