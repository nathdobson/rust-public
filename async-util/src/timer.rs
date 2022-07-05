use std::future::poll_fn;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

use lazy_static::lazy_static;
use priority_queue::PriorityQueue;
use serde::{Deserialize, Serialize};
use util::time::SerialInstant;
use waker_util::HashWaker;
use crate::poll::PollResult;
use crate::poll::PollResult::{Noop, Yield};

struct State {
    queue: PriorityQueue<HashWaker, SerialInstant>,
}

struct Timer {
    state: Mutex<State>,
    condvar: Condvar,
}

lazy_static! {
    static ref TIMER: Timer = timer();
}

fn timer() -> Timer {
    thread::spawn(move || {
        let mut lock = TIMER.state.lock().unwrap();
        loop {
            let now = SerialInstant::now();
            if let Some((_, &when)) = lock.queue.peek() {
                if when <= now {
                    lock.queue.pop().unwrap().0 .0.wake();
                } else {
                    lock = TIMER.condvar.wait_timeout(lock, when - now).unwrap().0;
                }
            } else {
                lock = TIMER.condvar.wait(lock).unwrap();
            }
        }
    });
    Timer {
        state: Mutex::new(State {
            queue: PriorityQueue::new(),
        }),
        condvar: Condvar::new(),
    }
}

pub fn poll_elapse(cx: &mut Context, instant: SerialInstant) -> Poll<()> {
    let now = SerialInstant::now();
    if instant <= now {
        Poll::Ready(())
    } else {
        let mut lock = TIMER.state.lock().unwrap();
        lock.queue.push(HashWaker(cx.waker().clone()), instant);
        TIMER.condvar.notify_one();
        Poll::Pending
    }
}

#[derive(Serialize, Deserialize)]
pub struct Sleep {
    time: Option<SerialInstant>,
    #[serde(skip)]
    waker: Option<Waker>,
}

impl Sleep {
    pub fn new() -> Self {
        Sleep {
            time: None,
            waker: None,
        }
    }
    pub fn set_instant(&mut self, time: SerialInstant) {
        self.time = Some(time);
        self.waker
            .as_ref()
            .map(|waker| poll_elapse(&mut Context::from_waker(waker), time));
    }
    pub fn set_delay(&mut self, delay: Duration) { self.set_instant(SerialInstant::now() + delay); }
    pub fn sleeping(&mut self) -> bool { self.time.is_some() }
    pub fn poll_sleep(&mut self, cx: &mut Context) -> PollResult {
        self.waker = Some(cx.waker().clone());
        if let Some(time) = self.time {
            if let Poll::Ready(()) = poll_elapse(cx, time) {
                self.time = None;
                Yield(())
            } else {
                Noop
            }
        } else {
            Noop
        }
    }
}

#[tokio::test]
async fn test() {
    let start = SerialInstant::now();
    let t1 = start + Duration::from_millis(100);
    let t2 = start + Duration::from_millis(200);
    let actions: Vec<Box<dyn FnOnce(&mut Context) + Send>> = vec![
        box |cx| {
            poll_elapse(cx, t2).is_ready();
            poll_elapse(cx, t1).is_ready();
        },
        box |cx| {
            assert!((SerialInstant::now() - t1).as_millis() < 50);
            poll_elapse(cx, t2).is_ready();
        },
        box |cx| {
            assert!((SerialInstant::now() - t2).as_millis() < 50);
            cx.waker().wake_by_ref()
        },
    ];
    let mut actions = actions.into_iter();
    poll_fn(|cx| {
        if let Some(action) = actions.next() {
            action(cx);
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;
}
