use std::time::Instant;
use crate::{Timer, Handler, TimerCallback};
use std::any::Any;
use std::sync::{Mutex, Arc, Condvar};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use util::watch::{Watchable, Watch};
use util::pmpsc;
use std::thread;

pub struct Sender {
    pub inner: pmpsc::Sender<Instant, TimerCallback>,
}

pub struct Receiver {
    pub inner: pmpsc::Receiver<Instant, TimerCallback>,
}

pub fn channel() -> (Sender, Receiver) {
    let (sender, receiver) = pmpsc::channel();
    (Sender { inner: sender }, Receiver { inner: receiver })
}

impl Receiver {
    pub fn start(self, handler: Arc<Mutex<dyn Handler>>) {
        thread::spawn(move || {
            while let Ok((_, callback)) = self.inner.recv_at_time() {
                callback(&mut *handler.lock().unwrap());
            }
        });
    }
}


impl Timer for Sender {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn schedule(&self, time: Instant, callback: TimerCallback) {
        self.inner.send(time, callback);
    }
}
