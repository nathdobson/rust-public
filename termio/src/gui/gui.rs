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
use std::raw::TraitObject;
use crate::gui::div::{Div, DivRc};
use crate::gui::tree::Tree;

const FRAME_BUFFER_SIZE: usize = 1;

pub struct Gui {
    style: Style,
    title: String,
    writer: TermWriter,
    size: (isize, isize),
    set_text_size_count: usize,
    tree: Tree,
    root: DivRc,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum InputEvent {
    MouseEvent {
        event: MouseEvent,
        inside: bool,
    },
    KeyEvent(KeyEvent),
}

impl Gui {
    pub fn new(tree: Tree, root: DivRc) -> Gui {
        let mut writer = TermWriter::new();
        writer.set_enabled(true);
        Gui {
            style: Default::default(),
            title: "".to_string(),
            writer,
            size: (0, 0),
            set_text_size_count: 0,
            tree,
            root,
        }
    }

    pub fn paint_buffer(&mut self, output: &mut Vec<u8>) {
        self.paint();
        output.clear();
        mem::swap(self.buffer(), output);
    }

    pub fn mark_dirty(&mut self){
        self.tree.mark_dirty();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.writer.set_enabled(enabled);
        self.mark_dirty();
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
            self.mark_dirty();
        }
    }

    pub fn set_title(&mut self, title: String) {
        if self.title != title {
            self.title = title;
            self.mark_dirty();
        }
    }

    pub fn buffer(&mut self) -> &mut Vec<u8> {
        self.writer.buffer()
    }

    pub fn layout(&mut self) {
        self.root.write().layout(&Constraint { max_size: Some(self.size) });
    }

    pub fn handle(&mut self, event: &Event) {
        match event {
            Event::KeyEvent(e) => {
                if *e == KeyEvent::typed('c').control() {
                    self.set_enabled(false);
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
                root.handle(&InputEvent::MouseEvent { event: event, inside: true });
            }
            Event::Focus(_) => {}
            Event::WindowPosition(_, _) => {}
            Event::WindowSize(_, _) => {}
            Event::TextAreaSize(w, h) => {
                self.set_text_size_count += 1;
                if self.size != (*w, *h) {
                    self.size = (*w, *h);
                    self.layout();
                    self.mark_dirty();
                }
            }
            Event::ScreenSize(_, _) => {}
        }
    }

    pub fn root(&self) -> &DivRc { &self.root }
    pub fn root_mut(&mut self) -> &mut DivRc { &mut self.root }
}

impl Debug for Gui {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Gui").finish()
    }
}
