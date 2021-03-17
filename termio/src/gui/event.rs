use util::{pmpsc, lossy};
use std::sync::{Arc};
//use util::any::Upcast;
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
use std::collections::VecDeque;
use std::future::Future;
use async_std::io::Read;
use std::io;
use futures::{pin_mut, StreamExt};
use futures::task::SpawnExt;
use async_std::task;
use std::task::{Context, Poll};
use crate::gui::div::{DivRc, Div, DivImpl};
use futures::channel::mpsc::UnboundedSender;
use futures::channel::mpsc;
use futures::stream::FusedStream;
use futures::future::poll_fn;
use futures::stream::Stream;
use std::pin::Pin;
use futures::FutureExt;
use async_util::priority::{priority_join2, PriorityPool};
use async_util::{Executor, priority};

#[must_use]
pub struct GuiEvent(Box<dyn FnOnce() + Send + Sync>);

#[derive(Clone)]
pub struct SharedGuiEvent(Arc<dyn Fn() + Send + Sync>);

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub enum GuiPriority {
    Action,
    Simulate,
    Read,
    Layout,
    Paint,
}

struct EventSenderInner {
    spawner: PriorityPool<GuiPriority>,
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

pub fn priority_consume<S: Stream>(x: S, mut f: impl FnMut(S::Item)) -> impl Future<Output=()> {
    async move {
        pin_mut!(x);
        poll_fn(|cx| {
            match x.as_mut().poll_next(cx) {
                Poll::Ready(Some(x)) => {
                    f(x);
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                Poll::Ready(None) => Poll::Ready(()),
                Poll::Pending => Poll::Pending,
            }
        }).await
    }
}


pub fn event_loop() -> (EventSender, impl Future<Output=()>) {
    let (spawner, runner) = priority::channel();
    (
        EventSender(Arc::new(EventSenderInner { spawner })),
        runner
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
        self.0.spawner.spawn(GuiPriority::Action, async { event.run() });
    }
    pub fn run_later(&self, event: GuiEvent) {
        self.0.spawner.spawn(GuiPriority::Simulate, async { event.run() });
    }
    pub fn spawner(&self) -> &PriorityPool<GuiPriority> {
        &self.0.spawner
    }
    pub fn run_with_delay(&self, delay: Duration, event: GuiEvent) {
        self.0.spawner.spawn(GuiPriority::Simulate, async move {
            task::sleep(delay).await;
            event.run();
        });
    }
    pub fn run_at(&self, instant: Instant, event: GuiEvent) {
        self.run_with_delay(
            instant.checked_duration_since(Instant::now()).unwrap_or_default(),
            event)
    }

    pub fn spawn_poll_div<T: DivImpl, F>(&self, mut poll: F, div: DivRc<T>)
        where F: FnMut(&mut Div<T>, &mut Context) -> Poll<()> + Send + 'static {
        let div = div.downgrade();
        self.0.spawner.spawn(GuiPriority::Simulate, async move {
            poll_fn(|cx| {
                if let Some(mut div) = div.upgrade() {
                    poll(&mut *div.write(), cx)
                } else {
                    Poll::Ready(())
                }
            }).await;
        });
    }
}

impl Debug for EventSenderInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventSenderInner").finish()
    }
}

impl Debug for SharedGuiEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedGuiEvent").finish()
    }
}

impl Debug for GuiEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("GuiEvent").finish()
    }
}