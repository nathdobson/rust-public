use std::time::{Instant, SystemTime, Duration, UNIX_EPOCH};
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use lazy_static::lazy_static;
use std::ops::{Deref, Sub, Add};
use std::{thread, mem, iter};
use crate::shared::ObjectInner;
use ondrop::OnDrop;
use std::cell::Cell;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Debug, Hash)]
pub struct SerialInstant(Instant);

thread_local! {
    static ORIGIN: Cell<Option<Instant>> = Cell::new(None);
}

impl SerialInstant {
    pub fn now() -> Self { SerialInstant(Instant::now()) }
    pub fn instant(&self) -> Instant { self.0 }
    pub fn elapsed(&self) -> Duration { self.0.elapsed() }
}

pub fn serial_scope() -> impl Drop {
    ORIGIN.with(|origin| {
        assert!(origin.get().is_none());
        origin.set(Some(Instant::now()));
    });
    OnDrop::new(|| {
        ORIGIN.with(|origin| {
            origin.set(None);
        });
    })
}

impl Serialize for SerialInstant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let origin = ORIGIN.with(|origin| origin.get().expect("Time origin must be defined"));
        let diff;
        if self.0 < origin {
            diff = -((origin - self.0).as_nanos() as i128);
        } else {
            diff = (self.0 - origin).as_nanos() as i128;
        }
        serializer.serialize_i128(diff)
    }
}

impl<'de> Deserialize<'de> for SerialInstant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where
        D: Deserializer<'de> {
        let origin = ORIGIN.with(|origin| origin.get().unwrap());
        let diff = i128::deserialize(deserializer)?;
        Ok(SerialInstant(if diff < 0 {
            origin - Duration::from_nanos((-diff) as u64)
        } else {
            origin + Duration::from_nanos(diff as u64)
        }))
    }
}

impl Sub for SerialInstant {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add<Duration> for SerialInstant {
    type Output = SerialInstant;

    fn add(self, rhs: Duration) -> Self::Output {
        SerialInstant(self.0 + rhs)
    }
}

#[test]
fn test() {
    let i1 = SerialInstant::now();
    thread::sleep(Duration::from_millis(100));
    let i2 = SerialInstant::now();
    let scope = serial_scope();
    let encoded = serde_json::to_string(&(i1, i2)).unwrap();
    mem::drop(scope);
    thread::sleep(Duration::from_millis(200));
    let scope = serial_scope();
    let (i3, i4): (SerialInstant, SerialInstant) = serde_json::from_str(&encoded).unwrap();
    mem::drop(scope);
    let d1 = (i3.instant() - i1.instant()).as_micros();
    let d2 = (i4.instant() - i2.instant()).as_micros();
    assert!(d1 > 190_000 && d1 < 210_000);
    assert!(d2 > 190_000 && d2 < 210_000);
}
