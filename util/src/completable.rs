use std::mem;
use std::sync::mpsc::RecvError;
use std::sync::{Arc, Condvar, Mutex};

#[doc(hidden)]
pub enum State<T> {
    Waiting,
    Ok(T),
    Err,
}

#[doc(hidden)]
pub type Inner<T> = Arc<(Mutex<State<T>>, Condvar)>;

pub struct Sender<T>(Option<Inner<T>>);

pub enum Receiver<T> {
    #[doc(hidden)]
    Waiting(Inner<T>),
    #[doc(hidden)]
    Ok(T),
    #[doc(hidden)]
    Err,
}

impl<T> Sender<T> {
    pub fn send(mut self, value: T) {
        if let Some(inner) = self.0.take() {
            let mut lock = inner.0.lock().unwrap();
            println!("Sending");
            *lock = State::Ok(value);
            inner.1.notify_one();
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if let Some(inner) = self.0.take() {
            let mut lock = inner.0.lock().unwrap();
            println!("Sending");
            *lock = State::Err;
            inner.1.notify_one();
        }
    }
}

impl<T> Receiver<T> {
    fn resolve(&mut self) {
        match self {
            Receiver::Waiting(inner) => {
                let mut lock = inner.0.lock().unwrap();
                loop {
                    match mem::replace(&mut *lock, State::Waiting) {
                        State::Ok(x) => {
                            mem::drop(lock);
                            *self = Receiver::Ok(x);
                            return;
                        }
                        State::Err => {
                            mem::drop(lock);
                            *self = Receiver::Err;
                            return;
                        }
                        State::Waiting => lock = inner.1.wait(lock).unwrap(),
                    }
                }
            }
            _ => {}
        }
    }
    pub fn get_mut(&mut self) -> Result<&mut T, RecvError> {
        self.resolve();
        match self {
            Receiver::Waiting(_) => unreachable!(),
            Receiver::Ok(x) => Ok(x),
            Receiver::Err => Err(RecvError),
        }
    }
    pub fn into_inner(mut self) -> Result<T, RecvError> {
        self.resolve();
        match self {
            Receiver::Waiting(_) => unreachable!(),
            Receiver::Ok(x) => Ok(x),
            Receiver::Err => Err(RecvError),
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner1 = Arc::new((Mutex::new(State::Waiting), Condvar::new()));
    let inner2 = inner1.clone();
    (Sender(Some(inner1)), Receiver::Waiting(inner2))
}

pub fn success<T>(x: T) -> Receiver<T> { Receiver::Ok(x) }

pub fn failure<T>() -> Receiver<T> { Receiver::Err }

#[test]
fn test_completable_ok() {
    let (sender, mut receiver) = channel();
    sender.send(1);
    assert_eq!(receiver.get_mut(), Ok(&mut 1));
    assert_eq!(receiver.into_inner(), Ok(1));
}

#[test]
fn test_completable_err() {
    let (sender, mut receiver) = channel::<!>();
    mem::drop(sender);
    assert!(receiver.get_mut().is_err());
    assert!(receiver.into_inner().is_err());
}
