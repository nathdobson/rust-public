use std::sync::{Mutex, Condvar, Arc};
use std::sync::mpsc::RecvError;
use std::mem;

enum State<T: Send> {
    Waiting,
    Ok(T),
    Err,
}

type Inner<T> = Arc<(Mutex<State<T>>, Condvar)>;

pub struct Sender<T: Send>(Inner<T>);

pub struct Receiver<T: Send>(Inner<T>);

impl<T: Send> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut lock = (self.0).0.lock().unwrap();
        *lock = State::Err;
        (self.0).1.notify_one();
    }
}

impl<T: Send> Sender<T> {
    pub fn send(&self, value: T) {
        let mut lock = (self.0).0.lock().unwrap();
        *lock = State::Ok(value);
        (self.0).1.notify_one();
    }
}

impl<T: Send> Receiver<T> {
    pub fn recv(&self) -> Result<T, RecvError> {
        let mut lock = (self.0).0.lock().unwrap();
        lock = (self.0).1.wait_while(lock, |x| match x {
            State::Waiting => true,
            _ => false
        }).unwrap();
        match &mut *lock {
            State::Waiting => unreachable!(),
            State::Ok(_) => {
                match mem::replace(&mut *lock, State::Waiting) {
                    State::Waiting => unreachable!(),
                    State::Ok(x) => return Ok(x),
                    State::Err => unreachable!(),
                }
            }
            State::Err => return Err(RecvError),
        }
    }
}

pub fn channel<T: Send>() -> (Sender<T>, Receiver<T>) {
    let inner1 = Inner::new((Mutex::new(State::Waiting), Condvar::new()));
    let inner2 = inner1.clone();
    (Sender(inner1), Receiver(inner2))
}

#[test]
fn test_lossy() {
    let (s, r) = channel();
    s.send(1);
    s.send(2);
    assert_eq!(r.recv(), Ok(2));
    mem::drop(s);
    assert!(r.recv().is_err());
}