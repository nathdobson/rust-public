use util::tree::Tree;
use util::rect::Rect;
use util::dynbag::Bag;
use crate::screen::{Style, LineSetting, Screen};
use crate::writer::TermWriter;
use std::any::{Any, TypeId};
use util::any::{Upcast, AnyExt};
use std::{fmt, thread, mem};
use std::sync::{Arc, mpsc, Mutex, Condvar, Weak};
use crate::input::{MouseEvent, KeyEvent, Event};
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
use crate::gui::node::{Node};
use crate::gui::view::{View, ViewImpl, UpcastGui};
use std::raw::TraitObject;

const FRAME_BUFFER_SIZE: usize = 1;

pub struct Gui<T: ?Sized = dyn ViewImpl> {
    style: Style,
    title: String,
    writer: TermWriter,
    size: (isize, isize),
    set_text_size_count: usize,
    root: View<T>,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum InputEvent {
    MouseEvent {
        event: MouseEvent,
        inside: bool,
    },
}

impl<T: ViewImpl> Gui<T> {
    pub fn new(root: View<T>) -> Gui<T> {
        let mut writer = TermWriter::new();
        writer.set_enabled(true);
        Gui {
            style: Default::default(),
            title: "".to_string(),
            writer,
            size: (0, 0),
            set_text_size_count: 0,
            root,
        }
    }
}

impl<T: ViewImpl + ?Sized> Gui<T> {
    pub fn downcast_gui<T2: ViewImpl>(&self) -> &Gui<T2> {
        unsafe {
            let this: &Gui = self.upcast_gui();
            let imp: &dyn ViewImpl = this.root.deref();
            let any: &dyn Any = imp.upcast();
            assert!(any.is::<T2>());
            let to: TraitObject = mem::transmute(this);
            let raw: *mut () = to.data;
            mem::transmute(raw)
        }
    }
    pub fn downcast_gui_mut<T2: ViewImpl>(&mut self) -> &mut Gui<T2> {
        unsafe {
            let this: &mut Gui = self.upcast_gui_mut();
            let imp: &mut dyn ViewImpl = this.root.deref_mut();
            let any: &mut dyn Any = imp.upcast_mut();
            assert!(any.is::<T2>());
            let to: TraitObject = mem::transmute(this);
            let raw: *mut () = to.data;
            mem::transmute(raw)
        }
    }

    pub fn paint_buffer(&mut self, output: &mut Vec<u8>) {
        self.paint();
        output.clear();
        mem::swap(self.buffer(), output);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.writer.set_enabled(enabled);
        self.root.mark_dirty();
    }

    pub fn enabled(&self) -> bool {
        self.writer.enabled()
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

    pub fn handle(&mut self, event: &Event) {
        match event {
            Event::KeyEvent(e) => {
                if *e == KeyEvent::typed('c').control() {
                    self.set_enabled(false);
                }
            }
            Event::MouseEvent(event) => {
                let mut event = event.clone();
                event.position.0 -= 1;
                event.position.1 -= 1;
                if let Some(line_setting) = self.root.line_setting(event.position.1) {
                    if line_setting != LineSetting::Normal {
                        event.position.0 *= 2;
                    }
                }
                self.root.handle(&InputEvent::MouseEvent { event: event, inside: true });
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
        }
    }

    pub fn root_mut(&mut self) -> &mut View<T> { &mut self.root }
    pub fn root(&self) -> &View<T> { &self.root }
}

impl<T: ViewImpl + ?Sized> Debug for Gui<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Gui").finish()
    }
}

impl<T: ?Sized> Deref for Gui<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.root.deref()
    }
}

impl<T: ?Sized> DerefMut for Gui<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.root.deref_mut()
    }
}