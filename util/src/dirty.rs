use std::thread::JoinHandle;
use std::{thread, mem};
use std::sync::{Arc, Mutex, Condvar};
//
//#[derive(Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug)]
//enum DirtyEnum {
//    CLEAN,
//    DIRTY,
//    CANCELLED,
//}
//
//#[derive(Clone)]
//struct DirtyState { inner: Arc<(Mutex<DirtyEnum>, Condvar)> }
//
//impl DirtyState {
//    pub fn new() -> Self {
//        DirtyState { inner: Arc::new((Mutex::new(DirtyEnum::CLEAN), Condvar::new())) }
//    }
//    fn mark_clean(&self) {
//        let mut lock = self.inner.0.lock().unwrap();
//        assert!(*lock != DirtyEnum::CANCELLED);
//        *lock = DirtyEnum::CLEAN;
//    }
//    fn mark_dirty(&self) {
//        let mut lock = self.inner.0.lock().unwrap();
//        assert!(*lock != DirtyEnum::CANCELLED);
//        *lock = DirtyEnum::DIRTY;
//        self.inner.1.notify_one();
//    }
//    fn mark_cancelled(&self) {
//        let mut lock = self.inner.0.lock().unwrap();
//        *lock = DirtyEnum::CANCELLED;
//        self.inner.1.notify_one();
//    }
//    fn wait_not_clean(&self) -> DirtyEnum {
//        let mut lock = self.inner.0.lock().unwrap();
//        lock = self.inner.1.wait_while(lock, |&mut x| x == DirtyEnum::CLEAN).unwrap();
//        *lock
//    }
//}
//
//pub struct DirtyLoop {
//    thread: Option<JoinHandle<()>>,
//    dirty: DirtyState,
//}
//
//pub struct Cleaned(DirtyState);
//
//impl DirtyLoop {
//    pub fn new(mut cleaner: Box<dyn FnMut() -> Cleaned + 'static + Send>) -> Self {
//        let dirty = DirtyState::new();
//        let dirty2 = dirty.clone();
//        let thread = Some(thread::spawn(move || {
//            while let DirtyEnum::DIRTY = dirty2.wait_not_clean() {
//                let Cleaned(cleaned) = cleaner();
//                assert!(Arc::ptr_eq(&cleaned.inner, &dirty2.inner));
//            }
//        }));
//        DirtyLoop { thread, dirty }
//    }
//    pub fn mark_dirty(&mut self) {
//        self.dirty.mark_dirty();
//    }
//    pub fn mark_clean(&mut self) -> Cleaned {
//        self.dirty.mark_clean();
//        Cleaned(self.dirty.clone())
//    }
//}
//
//impl Drop for DirtyLoop {
//    fn drop(&mut self) {
//        self.dirty.mark_cancelled();
//        self.thread.take().expect("thread").join().ok();
//    }
//}
//
//#[test]
//fn test_render_loop_once() {
//    use std::sync::mpsc::{channel, RecvTimeoutError};
//    use std::time::Duration;
//    use std::mem;
//
//    let container: Arc<Mutex<Option<DirtyLoop>>> = Arc::new(Mutex::new(None));
//    let container2 = Arc::downgrade(&container);
//    let mut lock = container.lock().unwrap();
//    let (s, r) = channel();
//    *lock = Some(DirtyLoop::new(Box::new(move || {
//        let clean = container2
//            .upgrade().expect("upgrade")
//            .lock().expect("lock")
//            .as_mut().expect("inner")
//            .mark_clean();
//        s.send(()).unwrap();
//        clean
//    })));
//    for _ in 0..2 {
//        lock.as_mut().unwrap().mark_dirty();
//    }
//    mem::drop(lock);
//    let timeout = Duration::from_secs(1);
//    assert_eq!(r.recv_timeout(timeout), Ok(()));
//    mem::drop(container);
//    assert_eq!(r.recv_timeout(timeout), Err(RecvTimeoutError::Disconnected));
//}
//
//#[test]
//fn test_render_loop_many() {
//    for _ in 0..100000 {
//        test_render_loop_once();
//    }
//}

struct State {
    fun: Option<Box<dyn FnOnce() + 'static + Send>>,
    cancelled: bool,
}

pub struct DirtyLoop {
    state: Arc<(Mutex<State>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl DirtyLoop {
    pub fn new() -> Self {
        let state = Arc::new((Mutex::new(State { fun: None, cancelled: false }), Condvar::new()));
        let state2 = state.clone();
        let thread = Some(thread::spawn(move || {
            loop {
                let mut lock = state2.0.lock().unwrap();
                lock = state2.1
                    .wait_while(lock, |lock|
                        {
                            lock.fun.is_none() && !lock.cancelled
                        }).unwrap();
                let fun = lock.fun.take();
                let cancelled = lock.cancelled;
                mem::drop(lock);
                if let Some(fun) = fun {
                    fun();
                }
                if cancelled {
                    return;
                }
            }
        }));
        DirtyLoop {
            state,
            thread,
        }
    }
    pub fn spawn(&self, f: Box<dyn FnOnce() + 'static + Send>) {
        let mut lock = self.state.0.lock().unwrap();
        lock.fun = Some(f);
        self.state.1.notify_one();
    }
    fn cancel(&self) {
        if let Ok(mut lock) = self.state.0.lock() {
            lock.cancelled = true;
            self.state.1.notify_one();
        }
    }
    pub fn cancel_and_join(mut self) -> thread::Result<()> {
        let thread = self.thread.take().unwrap();
        mem::drop(self);
        thread.join()
    }
}

impl Drop for DirtyLoop {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[test]
fn test_dirty() {
    use std::sync::mpsc::channel;
    use std::time::Duration;
    let dirty1 = Arc::new(DirtyLoop::new());
    let dirty2 = dirty1.clone();
    let (s, r) = channel();
    dirty1.spawn(Box::new(move || {
        dirty2.spawn(Box::new(move || {
            assert!(false);
        }));
        dirty2.spawn(Box::new(move || {
            s.send(()).unwrap();
        }))
    }));
    assert_eq!(r.recv_timeout(Duration::from_secs(1)), Ok(()));
}

#[test]
fn test_dirty_many(){
    for _ in 0..100000{
        test_dirty();
    }
}