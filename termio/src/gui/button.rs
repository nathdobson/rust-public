use crate::gui::node::{Node, NodeImpl, NodeId};
use crate::canvas::Canvas;
use crate::input::{MouseEvent, Mouse};
use crate::gui::gui::{OutputEvent, InputEvent};
use std::iter;
use std::collections::BTreeMap;
use crate::screen::{Style, LineSetting};
use crate::color::Color;
use crate::gui::layout::Constraint;
use std::sync::Arc;
use std::time::{Duration, Instant};
use crate::gui::time::TimeEvent;
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum PaintState {
    Normal,
    Over,
    Down,
}

#[derive(Debug)]
pub struct Button<T: ButtonPaint = TextButtonPaint> {
    event: OutputEvent,
    over: bool,
    down: bool,
    countdown: Option<Instant>,
    state: PaintState,
    paint: T,
}

#[derive(Debug)]
pub struct TextButtonPaint {
    text: String,
    normal_style: Style,
    over_style: Style,
    down_style: Style,
}

impl<T: ButtonPaint> Button<T> {
    pub fn new_from_paint(id: NodeId, paint: T, event: OutputEvent) -> Node<Self> {
        Node::new(id, Button {
            over: false,
            down: false,
            event,
            paint,
            countdown: None,
            state: PaintState::Normal,
        })
    }
}

impl Button {
    pub fn new(id: NodeId, text: String, event: OutputEvent) -> Node<Self> {
        Button::new_from_paint(id, TextButtonPaint::new(text), event)
    }
}

impl<T: ButtonPaint> Button<T> {
    pub fn state(&self) -> PaintState {
        self.state
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
            normal_style: Style {
                background: Color::Gray24(15),
                ..base
            },
            over_style: Style {
                background: Color::Gray24(18),
                ..base
            },
            down_style: Style {
                background: Color::Gray24(23),
                ..base
            },
        }
    }
}

pub trait ButtonPaint: Sized + Debug + Send + Sync {
    fn paint(this: &Node<Button<Self>>, canvas: Canvas);
    fn line_setting(this: &Node<Button<Self>>, row: isize) -> Option<LineSetting> { Some(LineSetting::Normal) }
    fn layout(this: &mut Node<Button<Self>>, constraint: &Constraint) -> (isize, isize);
}

impl ButtonPaint for TextButtonPaint {
    fn paint(this: &Node<Button<Self>>, mut w: Canvas) {
        w.style = match this.state() {
            PaintState::Normal => this.normal_style,
            PaintState::Over => this.over_style,
            PaintState::Down => this.down_style,
        };
        w.draw((0, 0),
               &iter::once('▛')
                   .chain(iter::repeat('▀').take(this.text.len()))
                   .chain(iter::once('▜')).collect::<String>());
        w.draw((0, 1),
               &format!("▌{}▐", this.text, ));
        w.draw((0, 2),
               &iter::once('▙')
                   .chain(iter::repeat('▄').take(this.text.len()))
                   .chain(iter::once('▟')).collect::<String>());
    }

    fn layout(this: &mut Node<Button<Self>>, constraint: &Constraint) -> (isize, isize) {
        ((this.text.len() + 2) as isize, 3)
    }
}

impl<T: ButtonPaint> NodeImpl for Button<T> {
    fn paint(self: &Node<Self>, w: Canvas) {
        T::paint(self, w)
    }

    fn handle(self: &mut Node<Self>, event: &InputEvent, output: &mut Vec<OutputEvent>) {
        match event {
            InputEvent::MouseEvent { event, inside } => {
                let was_down = self.down;
                let was_over = self.over;
                self.over = *inside;
                self.down = self.over && event.mouse == Mouse::Down(0);
                if was_down && !self.down && *inside {
                    output.push(self.event.clone());
                    let countdown = Instant::now() + Duration::from_millis(50);
                    self.countdown = Some(countdown);
                    output.push(TimeEvent::new(countdown));
                }
            }
            InputEvent::TimeEvent { when } => {
                if let Some(countdown) = self.countdown {
                    if *when >= countdown {
                        self.countdown = None;
                    }
                }
            }
        }
        let new_state =
            if self.down || self.countdown.is_some() {
                PaintState::Down
            } else if self.over {
                PaintState::Over
            } else {
                PaintState::Normal
            };
        if self.state != new_state {
            self.state = new_state;
            self.mark_dirty();
        }
    }

    fn layout(self: &mut Node<Self>, constraint: &Constraint) {
        let size = T::layout(self, constraint);
        self.set_size(size);
    }

    fn line_setting(self: &Node<Self>, row: isize) -> Option<LineSetting> {
        T::line_setting(self, row)
    }
}

impl<T: ButtonPaint> Deref for Button<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.paint }
}

impl<T: ButtonPaint> DerefMut for Button<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.paint }
}