use util::{pmpsc, lossy};
use std::sync::{Arc};
use util::any::Upcast;
use std::{thread, mem, sync};
use std::time::{Duration, Instant};
use std::sync::mpsc::RecvError;
use std::fmt::{Debug};
use std::fmt::Formatter;
use std::fmt;
use crate::gui::tree::Tree;
use util::atomic_refcell::AtomicRefCell;
use crate::gui::gui::Gui;
use std::str;
use std::thread::JoinHandle;
use crate::input::EventReader;
use std::cell::RefCell;
use libc::close;
use util::mutrc::MutRc;
use async_util::{Mutex, Condvar, Executor};
use std::collections::VecDeque;
use std::future::Future;
use async_std::io::Read;
use std::io;
use futures::pin_mut;
use futures::task::SpawnExt;
use async_std::task;

#[must_use]
pub struct GuiEvent(Box<dyn FnOnce() + Send + Sync>);

#[derive(Clone)]
pub struct SharedGuiEvent(Arc<dyn Fn() + Send + Sync>);


struct EventQueue {
    now: VecDeque<GuiEvent>,
    later: VecDeque<GuiEvent>,
}

struct EventSenderInner {
    mutex: Mutex,
    exec: Executor,
    queue: MutRc<EventQueue>,
    available: Condvar,
}

#[derive(Clone, Debug)]
pub struct EventSender(Arc<EventSenderInner>);

impl GuiEvent {
    pub fn new(f: impl 'static + Send + Sync + FnOnce()) -> Self {
        GuiEvent(Box::new(f))
    }
}

impl SharedGuiEvent {
    pub fn new(f: impl 'static + Fn() + Send + Sync) -> Self {
        SharedGuiEvent(Arc::new(f))
    }
    pub fn once(&self) -> GuiEvent {
        let this = self.clone();
        GuiEvent::new(move || (this.0)())
    }
}

impl GuiEvent {
    fn run(self) {
        (self.0)();
    }
}

pub fn event_loop(mutex: Mutex, exec: Executor) -> (EventSender, impl Future<Output=()>) {
    let queue = MutRc::new(EventQueue {
        now: Default::default(),
        later: Default::default(),
    });
    let available = Condvar::new();

    let event_loop = {
        let mutex = mutex.clone();
        let available = available.downgrade();
        let mut queue = queue.clone();
        async move {
            let mut lock = mutex.lock().await;
            loop {
                if let Some(now) = {
                    let x = queue.write().now.pop_front();
                    x
                } {
                    now.run();
                    continue;
                }
                if let Some(now) = {
                    let x = queue.write().later.pop_front();
                    x
                } {
                    now.run();
                    continue;
                }
                let l2 = lock;
                match available.wait(l2).await {
                    Ok(l) => { lock = l; }
                    Err(_) => break,
                }
            }
        }
    };
    (
        EventSender(Arc::new(EventSenderInner { mutex, queue, available, exec })),
        event_loop
    )
}

pub async fn read_loop(
    event_sender: EventSender,
    gui: MutRc<Gui>,
    read: impl 'static + Send + Read) -> io::Result<!> {
    let reader = EventReader::new(read);
    pin_mut!(reader);
    loop {
        let next = reader.as_mut().read().await?;
        let gui = gui.clone();
        let now = Instant::now();
        event_sender.run_later(GuiEvent::new(move || {
            gui.borrow_mut().handle(&next)
        }))
    }
}

impl EventSender {
    pub fn run_now(&self, event: GuiEvent) {
        self.0.queue.borrow_mut().now.push_back(event);
        self.0.available.notify_one();
    }
    async fn run_with_guard(self, event: GuiEvent) {
        let lock = self.0.mutex.lock().await;
        self.0.queue.borrow_mut().later.push_back(event);
        self.0.available.notify_one();
        mem::drop(lock);
    }
    pub fn run_later(&self, event: GuiEvent) {
        self.0.exec.spawn(self.clone().run_with_guard(event)).unwrap();
    }

    pub fn run_with_delay(&self, delay: Duration, event: GuiEvent) {
        let this=self.clone();
        self.0.exec.spawn(async move{
            task::sleep(delay).await;
            this.run_with_guard(event).await;
        }).unwrap();
    }

    pub fn run_at(&self, instant: Instant, event: GuiEvent) {
        self.run_with_delay(
            instant.checked_duration_since(Instant::now()).unwrap_or_default(),
            event)
    }

    pub fn mutex(&self) -> &Mutex {
        &self.0.mutex
    }
}

impl Debug for EventSenderInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventSenderInner").finish()
    }
}
