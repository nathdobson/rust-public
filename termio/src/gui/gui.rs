use crate::gui::node::Node;
use util::tree::Tree;
use crate::gui::container::Container;
use util::rect::Rect;
use util::dynbag::Bag;
use crate::screen::{Style, LineSetting, Screen};
use crate::writer::TermWriter;
use std::any::{Any, TypeId};
use util::any::Upcast;
use std::fmt;
use std::sync::Arc;
use crate::input::{MouseEvent, Event};
use crate::canvas::Canvas;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use crate::gui::layout::Constraint;

pub struct Gui<T: Node> {
    root: T,
    style: Style,
    title: String,
    writer: TermWriter,
    size: (isize, isize),
}

pub trait OutputEventTrait: Any + 'static + Send + Sync + fmt::Debug + Upcast<dyn Any> {}

pub type OutputEvent = Arc<dyn OutputEventTrait>;

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum InputEvent {
    MouseEvent {
        event: MouseEvent,
        inside: bool,
    },
}


impl<T: Node> Gui<T> {
    pub fn new(root: T) -> Self {
        let mut writer = TermWriter::new();
        writer.set_enabled(true);
        Gui {
            root,
            style: Default::default(),
            title: "".to_string(),
            writer,
            size: (0, 0),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.writer.set_enabled(enabled);
    }

    pub fn update_text_size(&mut self) {
        self.writer.get_text_size();
    }

    pub fn paint(&mut self) {
        if !self.root.check_dirty() {
            return;
        }
        let mut screen = Screen::new();
        screen.title = self.title.clone();
        for y in 1..self.size.1 {
            if let Some(line_setting) = self.root.line_setting(y) {
                screen.row(y as isize).line_setting = line_setting;
            }
        }
        let bounds = Rect::from_position_size((1, 1), self.size);
        self.writer.set_bounds(bounds);
        let canvas = Canvas::new(&mut screen, bounds, (1, 1), self.style);
        self.root.paint(canvas);
        self.writer.render(&screen, &self.style);
    }

    pub fn set_background(&mut self, style: Style) {
        if self.style != style {
            self.style = style;
            self.root.header_mut().mark_dirty();
        }
    }

    pub fn set_title(&mut self, title: String) {
        if self.title != title {
            self.title = title;
            self.root.header_mut().mark_dirty();
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
                if self.size != (*w, *h) {
                    self.size = (*w, *h);
                    self.layout();
                }
            }
            Event::ScreenSize(_, _) => {}
        }
    }
}