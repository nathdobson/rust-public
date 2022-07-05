//use util::any::{Upcast};
use std::any::{Any, TypeId};
use std::borrow::BorrowMut;
use std::collections::{HashMap, VecDeque};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::task::Context;
use std::time::{Duration, Instant};
use std::{fmt, io, mem, thread};

use async_util::delay_writer::DelayWriter;
use async_util::poll::PollResult::{Abort, Noop};
use async_util::poll::{poll_next, PollResult};
use async_util::timer::Sleep;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};
use util::lossy;
use util::rect::Rect;

use crate::canvas::Canvas;
use crate::gui::div::{Div, DivRc};
use crate::gui::layout::Constraint;
use crate::gui::tree::{Dirty, Tree, TreeReceiver};
use crate::gui::BoxAsyncWrite;
use crate::input::{Event, KeyEvent, MouseEvent};
use crate::screen::{LineSetting, Screen, Style};
use crate::writer::TermWriter;

const FRAME_BUFFER_SIZE: usize = 1;

pub struct Gui {
    style: Style,
    title: String,
    writer: TermWriter,
    size: (isize, isize),
    set_text_size_count: usize,
    tree: Tree,
    tree_receiver: TreeReceiver,
    event_receiver: Option<ReceiverStream<Event>>,
    output: BoxAsyncWrite,
    root: DivRc,
    resize_timeout: Sleep,
}

fn is_send() -> impl Send { Option::<Gui>::None }

fn is_sync() -> impl Sync { Option::<Gui>::None }

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum InputEvent {
    MouseEvent { event: MouseEvent, inside: bool },
    KeyEvent(KeyEvent),
}

impl Gui {
    pub fn new(
        tree: Tree,
        tree_receiver: TreeReceiver,
        event_receiver: mpsc::Receiver<Event>,
        output: BoxAsyncWrite,
        root: DivRc,
    ) -> Gui {
        let mut writer = TermWriter::new();
        writer.set_enabled(true);
        let event_receiver = Some(ReceiverStream::new(event_receiver));
        Gui {
            style: Default::default(),
            title: "".to_string(),
            writer,
            size: (0, 0),
            set_text_size_count: 0,
            tree,
            tree_receiver,
            event_receiver,
            output,
            root,
            resize_timeout: Sleep::new(),
        }
    }

    pub fn mark_dirty(&mut self, dirty: Dirty) { self.tree.mark_dirty(dirty); }

    pub fn tree(&self) -> &Tree { &self.tree }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.writer.enabled() {
            self.writer.set_enabled(enabled);
            self.mark_dirty(Dirty::Paint);
        }
    }

    pub fn enabled(&self) -> bool { self.writer.enabled() }

    pub fn paint(&mut self) {
        if !self.writer.enabled() {
            return;
        }
        if self.set_text_size_count + FRAME_BUFFER_SIZE <= self.writer.get_text_size_count() {
            return;
        }
        self.resize_timeout.set_delay(Duration::from_millis(1000));
        self.writer.get_text_size();
        let mut screen = Screen::new(self.size, self.style);
        let root = self.root.read();
        screen.title = self.title.clone();
        for y in 1..self.size.1 {
            if let Some(line_setting) = root.line_setting(y - 1) {
                screen.rows[y as usize].line_setting = line_setting;
            }
        }
        let bounds = Rect::from_position_size((1, 1), self.size);
        self.writer.set_bounds(bounds);
        let canvas = Canvas::new(&mut screen, bounds, (1, 1), self.style);
        root.paint(canvas);
        self.writer.render(&screen);
    }

    pub fn set_background(&mut self, style: Style) {
        if self.style != style {
            self.style = style;
            self.mark_dirty(Dirty::Paint);
        }
    }

    pub fn set_title(&mut self, title: String) {
        if self.title != title {
            self.title = title;
            self.mark_dirty(Dirty::Paint);
        }
    }

    pub fn writer(&mut self) -> &mut DelayWriter { self.writer.writer() }

    pub fn layout(&mut self) {
        self.root.write().layout(&Constraint {
            max_size: Some(self.size),
        });
        self.tree.mark_dirty(Dirty::Paint);
    }

    pub fn handle(&mut self, event: &Event) {
        match event {
            Event::KeyEvent(e) => {
                if *e == KeyEvent::typed('c').control() {
                    self.tree.cancel().cancel();
                } else {
                    let mut root = self.root.write();
                    root.handle(&InputEvent::KeyEvent(e.clone()));
                }
            }
            Event::MouseEvent(event) => {
                let mut root = self.root.write();
                let mut event = event.clone();
                event.position.0 -= 1;
                event.position.1 -= 1;
                if let Some(line_setting) = root.line_setting(event.position.1) {
                    if line_setting != LineSetting::Normal {
                        event.position.0 *= 2;
                    }
                }
                root.handle(&InputEvent::MouseEvent {
                    event: event,
                    inside: true,
                });
            }
            Event::Focus(_) => {}
            Event::WindowPosition(_, _) => {}
            Event::WindowSize(_, _) => {}
            Event::TextAreaSize(w, h) => {
                self.set_text_size_count += 1;
                if self.size != (*w, *h) {
                    self.size = (*w, *h);
                    self.mark_dirty(Dirty::Layout);
                }
            }
            Event::ScreenSize(_, _) => {}
            Event::CursorPosition(_, _) => {}
        }
    }

    pub fn root(&self) -> &DivRc { &self.root }
    pub fn root_mut(&mut self) -> &mut DivRc { &mut self.root }

    pub fn poll_elapse(&mut self, cx: &mut Context) -> PollResult<(), io::Error> {
        self.root.write().poll_elapse(cx)?;
        poll_next(cx, &mut self.event_receiver).map(|event| {
            self.handle(&event);
        })?;
        if self.event_receiver.is_none() {
            self.set_enabled(false);
        }
        poll_next(cx, &mut self.tree_receiver.layout).map(|()| {
            self.layout();
        })?;
        self.resize_timeout.poll_sleep(cx).map(|()| {
            self.mark_dirty(Dirty::Paint);
        })?;
        poll_next(cx, &mut self.tree_receiver.paint).map(|()| {
            self.paint();
        })?;
        self.writer
            .writer()
            .poll_flush(cx, Pin::new(&mut self.output))?;
        if self.event_receiver.is_none() && self.writer.writer().is_empty() {
            return Abort(io::Error::new(io::ErrorKind::Interrupted, "canceled"));
        }
        Noop
    }
}

impl Debug for Gui {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { f.debug_struct("Gui").finish() }
}
