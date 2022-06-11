use std::sync::mpsc::RecvError;

use crate::watch::{Watch, Watchable};

pub struct Sender<T: Send>(Watchable<Option<T>>);

pub struct Receiver<T: Send>(Watch<Option<T>>);

impl<T: Send> Sender<T> {
    pub fn send(&self, value: T) { **self.0.lock().unwrap() = Some(value); }
}

impl<T: Send> Receiver<T> {
    pub fn recv(&mut self) -> Result<T, RecvError> {
        loop {
            if let Some(result) = self.0.next().map_err(|_| RecvError)?.take() {
                return Ok(result);
            }
        }
    }
}

impl<T: Send> Iterator for Receiver<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> { self.recv().ok() }
}

pub fn channel<T: Send>() -> (Sender<T>, Receiver<T>) {
    let writer = Watchable::new(None);
    let reader = writer.watch();
    (Sender(writer), Receiver(reader))
}

#[test]
fn test_lossy() {
    use std::mem;
    let (s, mut r) = channel();
    s.send(1);
    s.send(2);
    assert_eq!(r.recv(), Ok(2));
    mem::drop(s);
    assert!(r.recv().is_err());
}
