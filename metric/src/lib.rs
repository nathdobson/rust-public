#![feature(test)]
#![feature(trait_alias)]
#![feature(result_into_ok_or_err)]
#![feature(never_type)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use parking_lot::Mutex;
use std::sync::Arc;
use rusqlite::Connection;
use crate::values::Values;

mod values;
mod sum;
mod histogram;
mod table;
mod database;
mod metric;
mod metric_set;
mod keys;


// impl<S: Stats + Clone> Metric<S> {
//     pub fn new(stats: S) -> Self {
//         let receiver = Arc::new(Mutex::new(StatsReceiver::new(stats)));
//         Database::add_callback({
//             let receiver = Arc::downgrade(&receiver);
//             let mut received = S::Set::default();
//             Box::new(move |conn| {
//                 if let Some(receiver) = receiver.upgrade() {
//                     received.clone_from(receiver.lock().recv());
//                     println!("{:?}", received);
//                     todo!()
//                 }
//             })
//         });
//         Metric { receiver }
//     }
//     pub fn local(&self) -> StatsSender<S> {
//         self.receiver.lock().sender()
//     }
// }


#[cfg(test)]
mod test {
    use crate::histogram::{Buckets, Point};
    use serde::Serialize;
    use serde::Deserialize;
    use lazy_static::lazy_static;
    use parking_lot::Mutex;
    use rusqlite::Connection;
    use std::thread::sleep;
    use std::time::Duration;
    use crate::metric::Metric;
    use crate::metric::LocalMetric;
    use crate::metric_set::MetricKey;
    use crate::metric_set::MetricSet;
    use std::sync::Arc;
    use std::any::Any;
    use std::ops::Deref;

    type KeysType = (&'static str, u32);
    type ValuesType = Buckets;

    static NAME: MetricKey = MetricKey::new("/example/metric");

    fn make_values() -> ValuesType {
        Buckets::exponential(1.0, 2.0, 10)
    }
    lazy_static! {
        static ref VALUES: ValuesType = make_values();
    }
    thread_local! {
        static LOCAL_METRIC: LocalMetric<KeysType, &'static ValuesType> =
            MetricSet::global().get_local(NAME, &&*VALUES);
    }
    fn add(field1: &'static str, field2: u32, value: f64) {
        LOCAL_METRIC.with(|local| local.add(
            (field1, field2),
            Point { value, weight: 1.0 },
        ));
    }

    #[test]
    fn test() {
        let set = MetricSet::new();
        set.set_for_thread();
        add("a", 2, 1000.0);
        println!("{:?}", set.get(NAME).unwrap());
    }
}