use std::task::{Poll, Context, Waker};
use util::time::SerialInstant;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;
use futures::executor::block_on;
use std::time::{Duration, Instant};
use crate::waker::HashWaker;
use lazy_static::lazy_static;
use priority_queue::PriorityQueue;
use async_std::future::poll_fn;

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
                    lock.queue.pop().unwrap().0.0.wake();
                } else {
                    lock = TIMER.condvar.wait_timeout(lock, when - now).unwrap().0;
                }
            } else {
                lock = TIMER.condvar.wait(lock).unwrap();
            }
        }
    });
    Timer {
        state: Mutex::new(State { queue: PriorityQueue::new() }),
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

#[test]
fn test() {
    let start = SerialInstant::now();
    let t1 = start + Duration::from_millis(100);
    let t2 = start + Duration::from_millis(200);
    let actions: Vec<Box<dyn FnOnce(&mut Context) + Send>> = vec![
        box |cx| {
            poll_elapse(cx, t2).is_ready();
            poll_elapse(cx, t1).is_ready();
        },
        box |cx| {
            assert!((SerialInstant::now() - t1).as_millis() < 5);
            poll_elapse(cx, t2).is_ready();
        },
        box |cx| {
            assert!((SerialInstant::now() - t2).as_millis() < 5);
            cx.waker().wake_by_ref()
        }
    ];
    let mut actions = actions.into_iter();
    block_on(async {
        poll_fn(|cx| {
            if let Some(action) = actions.next() {
                action(cx);
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        }).await;
    });
}