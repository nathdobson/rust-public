use std::sync::{Mutex, mpsc, Arc};
use crate::gui::gui::{Gui};
use std::{thread, mem, fmt};
use std::fmt::{Formatter, Debug};
use std::time::{Duration, Instant};

struct ContextInner {
    event_sender: Mutex<mpsc::Sender<GuiEvent>>,
    timer: Mutex<timer::Timer>,
    mark_dirty: Box<dyn Fn() + Send + Sync>,
}

#[must_use]
pub struct GuiEvent(Box<dyn FnOnce(&mut Gui) + Send + Sync>);

#[derive(Clone)]
pub struct SharedGuiEvent(Arc<dyn Fn(&mut Gui) + Send + Sync>);

pub type Guard = timer::Guard;

#[derive(Clone, Debug)]
pub struct Context(Arc<ContextInner>);

pub struct EventReceiver(mpsc::Receiver<GuiEvent>);

impl GuiEvent {
    pub fn new(f: impl 'static + FnOnce(&mut Gui) + Send + Sync) -> Self {
        GuiEvent(Box::new(f))
    }
}

impl SharedGuiEvent {
    pub fn new(f: impl 'static + Fn(&mut Gui) + Send + Sync) -> Self {
        SharedGuiEvent(Arc::new(f))
    }
    pub fn once(&self) -> GuiEvent {
        let this = self.clone();
        GuiEvent::new(move |gui| (this.0)(gui))
    }
}

impl Context {
    pub fn new(mark_dirty: Box<dyn Fn() + Send + Sync>) -> (Context, EventReceiver) {
        let (event_sender, event_receiver) = mpsc::channel();
        (Context(Arc::new(ContextInner {
            event_sender: Mutex::new(event_sender),
            timer: Mutex::new(timer::Timer::new()),
            mark_dirty,
        })),
         EventReceiver(event_receiver),
        )
    }

    pub fn run(&self, event: GuiEvent) {
        self.0.event_sender.lock().unwrap().send(event).unwrap();
    }

    #[must_use]
    pub fn run_with_delay(&self, delay: Duration, event: GuiEvent) -> Guard {
        let mut event = Some(event);
        let this = self.clone();
        self.0.timer.lock().unwrap().schedule_with_delay(
            chrono::Duration::from_std(delay).unwrap(),
            move || { this.run(event.take().unwrap()) })
    }

    #[must_use]
    pub fn run_at(&self, instant: Instant, event: GuiEvent) -> Guard {
        self.run_with_delay(
            instant.checked_duration_since(Instant::now()).unwrap_or_default(),
            event)
    }

    pub fn mark_dirty(&self) {
        (self.0.mark_dirty)()
    }
}

impl EventReceiver {
    pub fn start(self, gui: Arc<Mutex<Gui>>) {
        thread::spawn(move || {
            let mut lock = gui.lock().unwrap();
            loop {
                let event;
                if let Ok(e) = self.0.try_recv() {
                    event = e;
                } else {
                    mem::drop(lock);
                    if let Ok(e) = self.0.recv() {
                        lock = gui.lock().unwrap();
                        event = e;
                    } else {
                        break;
                    }
                }
                (event.0)(&mut *lock)
            }
        });
    }
}

impl Debug for ContextInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextInner").finish()
    }
}