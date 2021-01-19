use util::pmpsc;
use std::sync::{Arc, Mutex};
use util::any::Upcast;
use timer::Timer;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc::RecvError;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt;


#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum Priority {
    Now,
    Later,
}

pub type Guard = timer::Guard;

#[must_use]
pub struct GuiEvent(Box<dyn FnOnce(&mut dyn Controller) + Send + Sync>);

#[derive(Clone)]
pub struct SharedGuiEvent(Arc<dyn Fn(&mut dyn Controller) + Send + Sync>);

#[derive(Debug)]
pub struct EventSequence;

pub type EventMutex = Arc<Mutex<EventSequence>>;

#[must_use]
pub struct EventReceiver {
    event_receiver: pmpsc::Receiver<Priority, GuiEvent>,
    event_mutex: EventMutex,
}

struct EventSenderInner {
    timer: Timer,
    event_sender: pmpsc::Sender<Priority, GuiEvent>,
}

#[derive(Clone, Debug)]
pub struct EventSender(Arc<Mutex<EventSenderInner>>);

impl GuiEvent {
    pub fn new_dyn(f: impl 'static + Send + Sync + FnOnce(&mut dyn Controller)) -> Self {
        GuiEvent(Box::new(f))
    }
    pub fn new<C: Controller>(f: impl 'static + Send + Sync + FnOnce(&mut C)) -> Self {
        GuiEvent(Box::new(|c: &mut dyn Controller| { f(c.upcast_mut().downcast_mut().unwrap()) }))
    }
}

impl SharedGuiEvent {
    pub fn new_dyn(f: impl 'static + Fn(&mut dyn Controller) + Send + Sync) -> Self {
        SharedGuiEvent(Arc::new(f))
    }
    pub fn new<C: Controller>(f: impl 'static + Send + Sync + Fn(&mut C)) -> Self {
        SharedGuiEvent(Arc::new(move |c: &mut dyn Controller| { f(c.upcast_mut().downcast_mut().unwrap()) }))
    }
    pub fn once(&self) -> GuiEvent {
        let this = self.clone();
        GuiEvent::new_dyn(move |controller| (this.0)(controller))
    }
}

impl GuiEvent {
    fn run(self, controller: &mut dyn Controller) {
        (self.0)(controller);
    }
}

pub fn channel() -> (MainMutex, EventSender, EventReceiver) {
    let event_mutex = Arc::new(Mutex::new(EventSequence));
    let (event_sender, event_receiver) = pmpsc::channel();
    let timer = Timer::new();
    (
        mutex,
        EventSender(Arc::new(Mutex::new(EventSenderInner {
            timer,
            event_sender,
        }))),
        EventReceiver { event_mutex, event_receiver },
    )
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

impl EventReceiver {
    pub fn start(self) {
        thread::spawn(move || {
            loop {
                if let Err(RecvError) = self.0.peek() {
                    break;
                }
                let mut controller = controller.lock().unwrap();
                while let Ok(event) = self.0.try_recv() {
                    event.1.run(&mut *controller);
                }
            }
        });
    }
}

impl Debug for EventSenderInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventSenderInner").finish()
    }
}
