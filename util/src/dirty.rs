use std::thread;
use crate::lossy;


pub struct DirtyLoop(lossy::Sender<()>);

impl DirtyLoop {
    pub fn new(mut callback: Box<dyn FnMut() + 'static + Send>) -> Self {
        let (sender, receiver) = lossy::channel();
        thread::spawn(move || {
            while let Ok(()) = receiver.recv() {
                callback();
            }
        });
        DirtyLoop(sender)
    }
    pub fn run(&self) {
        self.0.send(());
    }
}

#[test]
fn test_dirty() {
    use std::sync::Barrier;
    use std::sync::Arc;

    let b1 = Arc::new(Barrier::new(2));
    let b2 = b1.clone();
    let dirty = DirtyLoop::new(Box::new(move || {
        b2.wait();
        b2.wait();
    }));
    dirty.run();
    b1.wait();
    dirty.run();
    dirty.run();
    b1.wait();
    b1.wait();
    b1.wait();
}

#[test]
fn test_dirty_many() {
    for _ in 0..100000 {
        test_dirty();
    }
}