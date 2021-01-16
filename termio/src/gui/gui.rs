use crate::gui::node::{Node, NodeImpl, NodeId};
use util::tree::Tree;
use util::rect::Rect;
use util::dynbag::Bag;
use crate::screen::{Style, LineSetting, Screen};
use crate::writer::TermWriter;
use std::any::{Any, TypeId};
use util::any::{Upcast, AnyExt};
use std::{fmt, thread, mem};
use std::sync::{Arc, mpsc, Mutex, Condvar, Weak};
use crate::input::{MouseEvent, Event};
use crate::canvas::Canvas;
use std::borrow::BorrowMut;
use std::collections::{HashMap, VecDeque};
use crate::gui::layout::Constraint;
use std::ops::{Deref, DerefMut};
use std::time::Instant;
use std::sync::atomic::AtomicBool;
use std::fmt::Debug;
use serde::export::Formatter;
use util::lossy;

const FRAME_BUFFER_SIZE: usize = 1;

pub struct Gui {
    root: Box<Node>,
    style: Style,
    title: String,
    writer: TermWriter,
    size: (isize, isize),
    set_text_size_count: usize,
}

pub type GuiEvent = Box<dyn FnOnce(&mut Gui) + Send + Sync>;

struct ContextInner {
    event_sender: Mutex<mpsc::Sender<GuiEvent>>,
    mark_dirty: Box<Fn() + Send + Sync>,
}

#[derive(Clone, Debug)]
pub struct Context(Arc<ContextInner>);

pub struct EventReceiver(mpsc::Receiver<GuiEvent>);

pub trait OutputEventTrait: Any + 'static + Send + Sync + fmt::Debug + Upcast<dyn Any> {}

pub type OutputEvent = Arc<dyn OutputEventTrait>;

impl dyn OutputEventTrait {
    pub fn downcast_event<T: 'static>(&self) -> util::any::Result<&T> {
        self.downcast_ref_result()
    }
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum InputEvent {
    MouseEvent {
        event: MouseEvent,
        inside: bool,
    },
    TimeEvent {
        when: Instant,
    },
}

impl Context {
    pub fn new(mark_dirty: Box<dyn Fn() + Send + Sync>) -> (Context, EventReceiver) {
        let (event_sender, event_receiver) = mpsc::channel();
        (Context(Arc::new(ContextInner {
            event_sender: Mutex::new(event_sender),
            mark_dirty,
        })),
         EventReceiver(event_receiver),
        )
    }
    pub fn run(&self, event: GuiEvent) {
        self.0.event_sender.lock().unwrap().send(event).unwrap();
    }
    pub fn mark_dirty(&self) {
        (self.0.mark_dirty)()
    }
}

impl EventReceiver {
    fn start(self, gui: Arc<Mutex<Gui>>) {
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
                event(&mut *lock)
            }
        });
    }
}

impl Gui {
    pub fn new(root: Box<Node>) -> Gui {
        let mut writer = TermWriter::new();
        writer.set_enabled(true);
        Gui {
            root,
            style: Default::default(),
            title: "".to_string(),
            writer,
            size: (0, 0),
            set_text_size_count: 0,
        }
    }

    pub fn paint_buffer(&mut self, output: &mut Vec<u8>) {
        self.paint();
        output.clear();
        mem::swap(self.buffer(), output);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.writer.set_enabled(enabled);
    }

    pub fn paint(&mut self) {
        if !self.writer.enabled() {
            return;
        }
        if self.set_text_size_count + FRAME_BUFFER_SIZE <= self.writer.get_text_size_count() {
            return;
        }
        self.writer.get_text_size();
        let mut screen = Screen::new(self.size, self.style);
        screen.title = self.title.clone();
        for y in 1..self.size.1 {
            if let Some(line_setting) = self.root.line_setting(y - 1) {
                screen.rows[y as usize].line_setting = line_setting;
            }
        }
        let bounds = Rect::from_position_size((1, 1), self.size);
        self.writer.set_bounds(bounds);
        let canvas = Canvas::new(&mut screen, bounds, (1, 1), self.style);
        self.root.paint(canvas);
        self.writer.render(&screen);
    }

    pub fn set_background(&mut self, style: Style) {
        if self.style != style {
            self.style = style;
            self.root.mark_dirty();
        }
    }

    pub fn set_title(&mut self, title: String) {
        if self.title != title {
            self.title = title;
            self.root.mark_dirty();
        }
    }

    pub fn buffer(&mut self) -> &mut Vec<u8> {
        self.writer.buffer()
    }

    pub fn layout(&mut self) {
        self.root.layout(&Constraint { max_size: Some(self.size) });
    }

    pub fn handle(&mut self, event: &Event, output: &mut Vec<OutputEvent>) {
        match event {
            Event::KeyEvent(_) => {}
            Event::MouseEvent(event) => {
                let mut event = event.clone();
                event.position.0 -= 1;
                event.position.1 -= 1;
                if let Some(line_setting) = self.root.line_setting(event.position.1) {
                    if line_setting != LineSetting::Normal {
                        event.position.0 *= 2;
                    }
                }
                self.root.handle(&InputEvent::MouseEvent { event: event, inside: true }, output);
            }
            Event::Focus(_) => {}
            Event::WindowPosition(_, _) => {}
            Event::WindowSize(_, _) => {}
            Event::TextAreaSize(w, h) => {
                self.set_text_size_count += 1;
                if self.size != (*w, *h) {
                    self.size = (*w, *h);
                    self.layout();
                    self.root.mark_dirty();
                }
            }
            Event::ScreenSize(_, _) => {}
            Event::Time(when) => {
                self.root.handle(&InputEvent::TimeEvent { when: *when }, output);
            }
        }
    }
    pub fn node(&self, id: NodeId) -> &Node {
        fn rec(node:&Node,id:NodeId)->&Node{

        }
        rec(&self.root, id)
    }
}

impl Debug for Gui {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Gui").finish()
    }
}

impl Debug for ContextInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextInner").finish()
    }
}