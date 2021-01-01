use crate::gui::node::{Node, NodeHeader};
use crate::canvas::Canvas;
use crate::input::{MouseEvent, Mouse};
use crate::gui::gui::{OutputEvent, InputEvent};
use std::iter;
use std::collections::BTreeMap;
use crate::screen::{Style, LineSetting};
use crate::color::Color;
use crate::gui::layout::Constraint;

pub struct Button<T: ButtonPaint = TextButtonPaint> {
    header: NodeHeader,
    event: OutputEvent,
    over: bool,
    down: bool,
    paint: T,
}

pub struct TextButtonPaint {
    text: String,
    normal: Style,
    over: Style,
    down: Style,
}

impl<T: ButtonPaint> Button<T> {
    pub fn new_from_paint(paint: T, event: OutputEvent) -> Self {
        Button {
            header: NodeHeader::new(),
            over: false,
            down: false,
            event,
            paint,
        }
    }
}

impl Button {
    pub fn new(text: String, event: OutputEvent) -> Self {
        Button::new_from_paint(TextButtonPaint::new(text), event)
    }
}

impl TextButtonPaint {
    fn new(text: String) -> Self {
        let base = Style {
            foreground: Color::RGB666(0, 0, 0),
            ..Style::default()
        };
        TextButtonPaint {
            text,
            normal: Style {
                background: Color::Gray24(15),
                ..base
            },
            over: Style {
                background: Color::Gray24(18),
                ..base
            },
            down: Style {
                background: Color::Gray24(23),
                ..base
            },
        }
    }
}

pub trait ButtonPaint {
    fn paint(&self, canvas: Canvas, over: bool, down: bool);
    fn layout(&self, constraint: &Constraint) -> (isize, isize);
}

impl ButtonPaint for TextButtonPaint {
    fn paint(&self, mut w: Canvas, over: bool, down: bool) {
        w.style = if down && over {
            self.down
        } else if over {
            self.over
        } else {
            self.normal
        };
        w.draw((0, 0),
               &iter::once('▛')
                   .chain(iter::repeat('▀').take(self.text.len()))
                   .chain(iter::once('▜')).collect::<String>());
        w.draw((0, 1),
               &format!("▌{}▐", self.text, ));
        w.draw((0, 2),
               &iter::once('▙')
                   .chain(iter::repeat('▄').take(self.text.len()))
                   .chain(iter::once('▟')).collect::<String>());
    }

    fn layout(&self, constraint: &Constraint) -> (isize, isize) {
        ((self.text.len() + 2) as isize, 3)
    }
}

impl<T: ButtonPaint> Node for Button<T> {
    fn paint(&self, w: Canvas) {
        self.paint.paint(w, self.over, self.down)
    }

    fn handle(&mut self, event: &InputEvent, output: &mut Vec<OutputEvent>) {
        match event {
            InputEvent::MouseEvent { event, inside } => {
                let was_down = self.down;
                let was_over = self.over;
                self.over = *inside;
                self.down = event.mouse == Mouse::Down(0);
                if was_down && !self.down && *inside {
                    output.push(self.event.clone());
                }
                if was_down != self.down || was_over != self.over {
                    self.header_mut().mark_dirty();
                }
            }
        }
    }

    fn layout(&mut self, constraint: &Constraint) {
        let size = self.paint.layout(constraint);
        self.header_mut().set_size(size);
    }

    fn header(&self) -> &NodeHeader { &self.header }

    fn header_mut(&mut self) -> &mut NodeHeader { &mut self.header }
}