use crate::gui::gui::{OutputEventTrait, OutputEvent};
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct TimeEvent(pub Instant);

impl OutputEventTrait for TimeEvent {}

impl TimeEvent {
    pub fn new(when: Instant) -> OutputEvent {
        Arc::new(TimeEvent(when))
    }
}

#[derive(Clone)]
pub struct Timer(Arc<Mutex<timer::Timer>>);

pub type Guard = timer::Guard;

impl Timer {
    pub fn new()->Self{
        Timer(Arc::new(Mutex::new(timer::Timer::new())))
    }

    #[must_use]
    pub fn schedule_with_delay(&self, delay: Duration, cb: impl 'static + FnOnce() + Send) -> Guard {
        let mut cb = Some(cb);
        self.0.lock().unwrap().schedule_with_delay(
            chrono::Duration::from_std(delay).unwrap(),
            move || { cb.take().unwrap()() })
    }

    #[must_use]
    pub fn schedule_at(&self, instant: Instant, cb: impl 'static + FnOnce() + Send) -> Guard {
        self.schedule_with_delay(
            instant.checked_duration_since(Instant::now()).unwrap_or_default(),
            cb)
    }
}