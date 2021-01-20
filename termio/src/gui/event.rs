use util::{pmpsc, lossy};
use std::sync::{Arc, Mutex};
use util::any::Upcast;
use timer::Timer;
use std::{thread, mem, io};
use std::time::{Duration, Instant};
use std::sync::mpsc::RecvError;
use std::fmt::{Debug};
use std::io::{Write, Read, stdin, stdout};
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
use std::os::unix::io::AsRawFd;
use util::mutrc::MutRc;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum Priority {
    Now,
    Later,
}

pub type Guard = timer::Guard;

#[must_use]
pub struct GuiEvent(Box<dyn FnOnce() + Send + Sync>);

#[derive(Clone)]
pub struct SharedGuiEvent(Arc<dyn Fn() + Send + Sync>);

#[derive(Debug)]
pub struct EventSequence;

pub type EventMutex = Arc<Mutex<EventSequence>>;

#[must_use]
pub struct DirtyReceiver {
    dirty_receiver: lossy::Receiver<()>,
}

struct EventSenderInner {
    timer: Timer,
    event_sender: pmpsc::Sender<Priority, GuiEvent>,
}

#[derive(Clone, Debug)]
pub struct EventSender(Arc<Mutex<EventSenderInner>>);

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

pub fn event_loop() -> (EventMutex, EventSender, JoinHandle<()>) {
    let event_mutex = Arc::new(Mutex::new(EventSequence));
    let (event_sender, event_receiver) =
        pmpsc::channel::<Priority, GuiEvent>();
    let timer = Timer::new();

    let event_joiner = thread::spawn({
        let event_mutex = event_mutex.clone();
        move || {
            loop {
                if let Err(RecvError) = event_receiver.peek() {
                    break;
                }
                let sequence = event_mutex.lock().unwrap();
                while let Ok(event) = event_receiver.try_recv() {
                    event.1.run();
                }
                mem::drop(sequence);
            }
        }
    });
    (
        event_mutex.clone(),
        EventSender(Arc::new(Mutex::new(EventSenderInner {
            timer,
            event_sender,
        }))),
        event_joiner
    )
}

pub fn render_loop(event_sender: EventSender) -> (Tree, DirtyReceiver) {
    let (dirty_sender, dirty_receiver) = lossy::channel();
    dirty_sender.send(());
    (Tree::new(event_sender, box move || dirty_sender.send(())), DirtyReceiver { dirty_receiver })
}

pub fn read_loop(
    event_sender: EventSender,
    gui: MutRc<Gui>,
    read: impl 'static + Send + Read) -> io::Result<!> {
    let mut reader = EventReader::new(read);
    loop {
        let next = reader.read()?;
        let gui = gui.clone();
        event_sender.run(Priority::Later, GuiEvent::new(move || {
            gui.borrow_mut().handle(&next)
        }))
    }
}

impl EventSender {
    pub fn run(&self, priority: Priority, event: GuiEvent) {
        self.0.lock().unwrap().event_sender.send(priority, event);
    }

    #[must_use]
    pub fn run_with_delay(&self, delay: Duration, event: GuiEvent) -> Guard {
        let mut event = Some(event);
        let this = self.clone();
        self.0.lock().unwrap().timer.schedule_with_delay(
            chrono::Duration::from_std(delay).unwrap(),
            move || { this.run(Priority::Later, event.take().unwrap()) })
    }

    #[must_use]
    pub fn run_at(&self, instant: Instant, event: GuiEvent) -> Guard {
        self.run_with_delay(
            instant.checked_duration_since(Instant::now()).unwrap_or_default(),
            event)
    }
}

impl DirtyReceiver {
    pub fn run(self,
               event_mutex: EventMutex,
               gui: MutRc<Gui>,
               mut write: impl 'static + Send + Write) {
        let mut buffer = vec![];
        for () in self.dirty_receiver {
            let enabled;
            {
                let _sequence = event_mutex.lock().unwrap();
                let mut gui = gui.borrow_mut();
                gui.paint_buffer(&mut buffer);
                enabled = gui.enabled();
            }
            {
                write.write_all(&buffer).unwrap();
                write.flush().unwrap();
            }
            if !enabled {
                break;
            }
        }
    }
}

impl Debug for EventSenderInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventSenderInner").finish()
    }
}

pub fn run_local(gui: impl FnOnce(Tree) -> MutRc<Gui>) {
    let (event_mutex, event_sender, event_joiner) = event_loop();
    let (tree, dirty_receiver) = render_loop(event_sender.clone());
    let gui = gui(tree.clone());
    thread::spawn({
        let gui = gui.clone();
        let event_sender = event_sender.clone();
        move || read_loop(event_sender, gui, stdin())
    });
    mem::drop(event_sender);
    mem::drop(event_joiner);
    mem::drop(tree);
    dirty_receiver.run(event_mutex, gui, stdout());
}