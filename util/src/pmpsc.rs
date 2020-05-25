use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex, Condvar};
use std::time::Instant;
use std::sync::mpsc::{RecvError, TryRecvError};
use std::thread;

struct Item<P: Ord, T> {
    id: usize,
    priority: P,
    value: T,
}

struct State<P: Ord, T> {
    next_id: usize,
    heap: BinaryHeap<Item<P, T>>,
    senders: usize,
}

struct Inner<P: Ord, T> {
    mutex: Mutex<State<P, T>>,
    condvar: Condvar,
}

pub struct Sender<P: Ord, T> {
    inner: Arc<Inner<P, T>>,
}

pub struct Receiver<P: Ord, T> {
    inner: Arc<Inner<P, T>>,
}

impl<P: Ord, T> Item<P, T> {
    fn key(&self) -> (Reverse<&P>, usize) {
        (Reverse(&self.priority), self.id)
    }
}

impl<P: Ord, T> PartialEq for Item<P, T> {
    fn eq(&self, other: &Self) -> bool { self.key().eq(&other.key()) }
}

impl<P: Ord, T> Eq for Item<P, T> {}

impl<P: Ord, T> PartialOrd for Item<P, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { self.key().partial_cmp(&other.key()) }
}

impl<P: Ord, T> Ord for Item<P, T> {
    fn cmp(&self, other: &Self) -> Ordering { self.key().cmp(&other.key()) }
}

pub fn channel<P: Ord, T>() -> (Sender<P, T>, Receiver<P, T>) {
    let sender = Arc::new(Inner {
        mutex: Mutex::new(State {
            next_id: 0,
            heap: BinaryHeap::new(),
            senders: 1,
        }),
        condvar: Condvar::new(),
    });
    let receiver = sender.clone();
    (Sender { inner: sender }, Receiver { inner: receiver })
}

impl<P: Ord, T> Sender<P, T> {
    pub fn send(&self, priority: P, value: T) {
        let mut lock = self.inner.mutex.lock().unwrap();
        let id = lock.next_id;
        lock.heap.push(Item { id, priority, value });
        lock.next_id += 1;
        self.inner.condvar.notify_one();
    }
}

impl<P: Ord, T> Drop for Sender<P, T> {
    fn drop(&mut self) {
        let mut lock = self.inner.mutex.lock().unwrap();
        lock.senders -= 1;
        if lock.senders == 0 {
            self.inner.condvar.notify_one();
        }
    }
}

impl<P: Ord, T> Clone for Sender<P, T> {
    fn clone(&self) -> Self {
        let mut lock = self.inner.mutex.lock().unwrap();
        lock.senders += 1;
        Sender { inner: self.inner.clone() }
    }
}

impl<P: Ord, T> Receiver<P, T> {
    pub fn try_recv(&self) -> Result<(P, T), TryRecvError> {
        let mut lock = self.inner.mutex.lock().unwrap();
        if lock.senders == 0 {
            Err(TryRecvError::Disconnected)
        } else if let Some(result) = lock.heap.pop() {
            Ok((result.priority, result.value))
        } else {
            Err(TryRecvError::Empty)
        }
    }
    pub fn recv(&self) -> Result<(P, T), RecvError> {
        let mut lock = self.inner.mutex.lock().unwrap();
        loop {
            if lock.senders == 0 {
                return Err(RecvError);
            } else if let Some(result) = lock.heap.pop() {
                return Ok((result.priority, result.value));
            } else {
                lock = self.inner.condvar.wait(lock).unwrap();
            }
        }
    }
}

impl<T> Receiver<Instant, T> {
    pub fn recv_at_time(&self) -> Result<(Instant, T), RecvError> {
        let mut lock = self.inner.mutex.lock().unwrap();
        loop {
            let now = Instant::now();
            if lock.senders == 0 {
                return Err(RecvError);
            }
            if let Some(next) = lock.heap.peek() {
                if next.priority <= now {
                    let result = lock.heap.pop().unwrap();
                    return Ok((result.priority, result.value));
                } else {
                    let timeout = next.priority - now;
                    lock = self.inner.condvar.wait_timeout(lock, timeout).unwrap().0;
                }
            } else {
                lock = self.inner.condvar.wait(lock).unwrap();
            }
        }
    }
}

impl Receiver<Instant, Box<dyn FnOnce() + Send + 'static>> {
    pub fn into_timer(self) {
        thread::spawn(move || {
            while let Ok((_, callback)) = self.recv_at_time() {
                callback();
            }
        });
    }
}


#[test]
fn test() {
    use std::time::Duration;
    use std::thread;
    use std::mem;

    let (sender, mut receiver) = channel();
    let handle = thread::spawn(move || {
        println!("b1");
        assert_eq!(receiver.recv_at_time().unwrap().1, 1);
        println!("b2");
        assert_eq!(receiver.recv_at_time().unwrap().1, 2);
        println!("b3");
        assert_eq!(receiver.recv_at_time().unwrap().1, 3);
        println!("b4");
        receiver
    });
    println!("a1");
    thread::sleep(Duration::from_millis(100));
    println!("a2");
    sender.send(Instant::now() + Duration::from_millis(100), 2);
    println!("a3");
    sender.send(Instant::now() + Duration::from_millis(200), 3);
    println!("a4");
    sender.send(Instant::now() + Duration::from_millis(50), 1);
    println!("a5");
    receiver = handle.join().unwrap();
    println!("a6");
    mem::drop(sender);
    println!("a7");
    assert_eq!(receiver.recv_at_time(), Err(RecvError));
    println!("a8");
}