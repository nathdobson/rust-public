use std::{thread, result, mem, fmt, io};
use crate::bag::{Bag, Token};
use std::sync::{Mutex, Condvar, Arc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::ops::Add;
use std::any::Any;

#[derive(Eq, Ord, PartialEq, PartialOrd, Debug)]
pub struct Cancel;

pub type Result<T> = result::Result<T, Cancel>;

#[derive(Debug)]
pub enum RecvError {
    Cancelling(Joiner),
    Empty(Receiver),
}

#[derive(Debug)]
pub enum JoinError {
    Panic,
    Empty(Joiner),
}

enum OnCancel {
    Pending(Box<dyn FnOnce() + 'static + Send>),
    Starting,
    Running(JoinHandle<()>),
}

pub struct OnCancelGuard(Option<(Context, Token)>);

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
enum Status {
    Active,
    Cancel,
    Panic,
}

#[derive(Debug)]
struct State {
    contexts: usize,
    status: Status,
    on_cancel: Bag<OnCancel>,
}

#[derive(Debug)]
pub struct Inner {
    mutex: Mutex<State>,
    condvar: Condvar,
}

pub struct Context {
    inner: Arc<Inner>,
}

#[derive(Clone)]
pub struct Canceller {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Receiver {
    inner: Arc<Inner>,
}

#[derive(Debug)]
pub struct Joiner {
    inner: Arc<Inner>,
}

impl fmt::Debug for OnCancel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OnCancel::Pending(_) => f.debug_struct("OnCancel::Pending").finish(),
            OnCancel::Starting => f.debug_struct("OnCancel::Starting").finish(),
            OnCancel::Running(_) => f.debug_struct("OnCancel::Running").finish(),
        }
    }
}

pub fn channel() -> (Context, Canceller, Receiver) {
    let context =
        Arc::new(Inner {
            mutex: Mutex::from(State {
                contexts: 1,
                status: Status::Active,
                on_cancel: Bag::new(),
            }),
            condvar: Condvar::new(),
        });
    let canceller = context.clone();
    let receiver = context.clone();
    (Context { inner: context },
     Canceller { inner: canceller },
     Receiver { inner: receiver })
}

impl Into<io::Error> for Cancel {
    fn into(self) -> io::Error {
        io::Error::new(ErrorKind::Interrupted, "Cancelled")
    }
}

impl From<Box<dyn Any + 'static + Send>> for JoinError {
    fn from(_: Box<dyn Any + 'static + Send>) -> Self {
        JoinError::Panic
    }
}

impl Inner {
    fn cancel(self: &Arc<Self>) {
        let mut lock = self.mutex.lock().unwrap();
        if lock.status == Status::Active {
            lock.status = Status::Cancel;
            for (_, on_cancel) in lock.on_cancel.iter_mut() {
                match mem::replace(on_cancel, OnCancel::Starting) {
                    OnCancel::Pending(callback) => {
                        *on_cancel = OnCancel::Running(thread::spawn(|| callback()));
                    }
                    _ => unreachable!(),
                }
            }
            self.condvar.notify_one();
        }
    }
}


impl Context {
    pub fn check(&self) -> Result<()> {
        let lock = self.inner.mutex.lock().unwrap();
        if lock.status == Status::Active {
            Ok(())
        } else {
            Err(Cancel)
        }
    }
    pub fn on_cancel(&self, callback: impl FnOnce() + Send + 'static) -> OnCancelGuard {
        if let Ok(mut lock) = self.inner.mutex.lock() {
            lock.contexts += 2;
            let self2 = Context { inner: self.inner.clone() };
            let self3 = Context { inner: self.inner.clone() };
            let callback = Box::new(|| {
                callback();
                mem::drop(self2)
            });
            let value = if lock.status == Status::Active {
                OnCancel::Pending(callback)
            } else {
                OnCancel::Running(thread::spawn(callback))
            };
            OnCancelGuard(Some((self3, lock.on_cancel.push(value))))
        } else {
            OnCancelGuard(None)
        }
    }
    pub fn spawn(&self, callback: impl FnOnce() -> Result<()> + Send + 'static) {
        if let Ok(mut lock) = self.inner.mutex.lock() {
            lock.contexts += 1;
            let context = Context { inner: self.inner.clone() };
            if lock.status == Status::Active {
                thread::spawn(|| {
                    if let Err(Cancel) = callback() {
                        context.inner.cancel();
                    } else {
                        mem::drop(context);
                    }
                });
            }
        }
    }
}

impl Drop for OnCancelGuard {
    fn drop(&mut self) {
        if let Some((context, token)) = self.0.take() {
            if let Ok(mut lock) = context.inner.mutex.lock() {
                match lock.on_cancel.remove(token) {
                    OnCancel::Pending(x) => {
                        mem::drop(lock);
                        mem::drop(x);
                    }
                    OnCancel::Running(x) => {
                        mem::drop(lock);
                        x.join().ok();
                        return;
                    }
                    _ => unreachable!()
                }
            }
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if let Ok(mut lock) = self.inner.mutex.lock() {
            if thread::panicking() {
                lock.status = Status::Panic;
            }
            lock.contexts -= 1;
            self.inner.condvar.notify_one();
        }
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        if let Ok(mut lock) = self.inner.mutex.lock() {
            lock.contexts += 1;
        }
        Context { inner: self.inner.clone() }
    }
}

impl Canceller {
    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

impl Receiver {
    pub fn recv(self) -> result::Result<(), RecvError> {
        self.recv_deadline(Instant::now().add(Duration::from_secs(60 * 60 * 24 * 365 * 100)))
    }
    pub fn try_recv(self) -> result::Result<(), RecvError> {
        self.recv_deadline(Instant::now())
    }
    pub fn recv_timeout(self, timeout: Duration) -> result::Result<(), RecvError> {
        self.recv_deadline(Instant::now().add(timeout))
    }
    pub fn recv_deadline(self, deadline: Instant) -> result::Result<(), RecvError> {
        let mut lock = self.inner.mutex.lock().unwrap();
        loop {
            if lock.status != Status::Active {
                mem::drop(lock);
                return Err(RecvError::Cancelling(Joiner { inner: self.inner }));
            }
            if lock.contexts == 0 {
                return Ok(());
            }
            let timeout = deadline.saturating_duration_since(Instant::now());
            let (lock2, timeout) =
                self.inner.condvar.wait_timeout(lock, timeout).unwrap();
            if timeout.timed_out() {
                return Err(RecvError::Empty(Receiver { inner: self.inner.clone() }));
            }
            lock = lock2;
        }
    }
}

impl Joiner {
    pub fn join(self) -> result::Result<(), JoinError> {
        self.join_deadline(Instant::now().add(Duration::from_secs(60 * 60 * 24 * 365 * 100)))
    }
    pub fn try_join(self) -> result::Result<(), JoinError> {
        self.join_deadline(Instant::now())
    }
    pub fn join_timeout(self, timeout: Duration) -> result::Result<(), JoinError> {
        self.join_deadline(Instant::now().add(timeout))
    }
    pub fn join_deadline(self, deadline: Instant) -> result::Result<(), JoinError> {
        let mut lock = self.inner.mutex.lock().unwrap();
        loop {
            if lock.status == Status::Panic {
                return Err(JoinError::Panic);
            }
            if lock.contexts == 0 {
                return Ok(());
            }
            let timeout = deadline.saturating_duration_since(Instant::now());
            let (lock2, timeout) =
                self.inner.condvar.wait_timeout(lock, timeout).unwrap();
            if timeout.timed_out() {
                return Err(JoinError::Empty(Joiner { inner: self.inner.clone() }));
            }
            lock = lock2;
        }
    }
}

#[cfg(test)]
const STEP: Duration = Duration::from_millis(100);

#[cfg(test)]
use std::sync::Barrier;
use std::io::ErrorKind;

#[test]
fn test_nothing() {
    let (context, _canceller, receiver) = channel();
    assert_eq!(context.check(), Ok(()));
    match receiver.recv_timeout(STEP) {
        Err(RecvError::Empty(_)) => {}
        unexpected => panic!("unexpect {:?}", unexpected),
    };
}

#[test]
fn test_return() {
    let (context, _canceller, receiver) = channel();
    mem::drop(context);
    assert_eq!(receiver.recv_timeout(STEP).ok(), Some(()));
}

#[test]
fn test_cancel() {
    let (context, canceller, receiver) = channel();
    canceller.cancel();
    assert_eq!(context.check(), Err(Cancel));
    let joiner = match receiver.recv_timeout(STEP) {
        Err(RecvError::Cancelling(joiner)) => joiner,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    mem::drop(context);
    assert!(joiner.join_timeout(STEP).is_ok());
}

#[test]
fn test_on_cancel_run() {
    let (context, canceller, receiver) = channel();
    let barrier = Arc::new(Barrier::new(2));
    let guard = context.on_cancel({
        let barrier = barrier.clone();
        move || {
            barrier.wait();
        }
    });
    canceller.cancel();
    let joiner = match receiver.recv_timeout(STEP) {
        Err(RecvError::Cancelling(joiner)) => joiner,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    barrier.wait();
    let joiner = match joiner.join_timeout(STEP) {
        Err(JoinError::Empty(joiner)) => joiner,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    mem::drop(guard);
    mem::drop(context);
    joiner.join_timeout(STEP).unwrap()
}

#[test]
fn test_on_cancel_drop() {
    let (context, canceller, receiver) = channel();
    let guard = context.on_cancel({
        move || {
            assert!(false);
        }
    });
    mem::drop(guard);
    canceller.cancel();
    let joiner = match receiver.recv_timeout(STEP) {
        Err(RecvError::Cancelling(joiner)) => joiner,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    let joiner = match joiner.join_timeout(STEP) {
        Err(JoinError::Empty(joiner)) => joiner,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    mem::drop(context);
    joiner.join_timeout(STEP).unwrap();
}

#[test]
fn test_on_cancel_panic() {
    let (context, canceller, receiver) = channel();
    let guard = context.on_cancel({
        move || {
            panic!();
        }
    });
    canceller.cancel();
    let joiner = match receiver.recv_timeout(STEP) {
        Err(RecvError::Cancelling(joiner)) => joiner,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    match joiner.join_timeout(STEP) {
        Err(JoinError::Panic) => {}
        unexpected => panic!("unexpect {:?}", unexpected),
    }
    mem::drop(context);
    mem::drop(guard);
}

#[test]
fn test_spawn_return() {
    let (context, _canceller, mut receiver) = channel();
    let barrier = Arc::new(Barrier::new(2));
    context.spawn({
        let barrier = barrier.clone();
        Box::new(move || {
            barrier.wait();
            Ok(())
        })
    });
    mem::drop(context);
    receiver = match receiver.recv_timeout(STEP) {
        Err(RecvError::Empty(receiver)) => receiver,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    barrier.wait();
    receiver.recv_timeout(STEP).unwrap();
}

#[test]
fn test_spawn_cancel() {
    let (context, canceller, receiver) = channel();
    let barrier = Arc::new(Barrier::new(2));
    context.spawn({
        let barrier = barrier.clone();
        Box::new(move || {
            barrier.wait();
            Ok(())
        })
    });
    mem::drop(context);
    canceller.cancel();
    let mut joiner = match receiver.recv_timeout(STEP) {
        Err(RecvError::Cancelling(joiner)) => joiner,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    joiner = match joiner.join_timeout(STEP) {
        Err(JoinError::Empty(joiner)) => joiner,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    barrier.wait();
    joiner.join_timeout(STEP).unwrap();
}

#[test]
fn test_spawn_panic() {
    let (context, _canceller, receiver) = channel();
    context.spawn({
        Box::new(move || {
            panic!();
        })
    });
    mem::drop(context);
    let joiner = match receiver.recv_timeout(STEP) {
        Err(RecvError::Cancelling(joiner)) => joiner,
        unexpected => panic!("unexpect {:?}", unexpected),
    };
    match joiner.join_timeout(STEP) {
        Err(JoinError::Panic) => {}
        unexpected => panic!("unexpect {:?}", unexpected),
    };
}


