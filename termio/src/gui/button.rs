use crate::canvas::Canvas;
use crate::input::{MouseEvent, Mouse};
use crate::gui::gui::{InputEvent};
use std::{iter, fmt};
use std::collections::{BTreeMap, HashMap};
use crate::screen::{Style, LineSetting};
use crate::color::Color;
use crate::gui::layout::{Constraint, Layout};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::ops::{Deref, DerefMut};
use std::fmt::Debug;
use crate::gui::view::{View, ViewImpl};
use crate::gui::node::{Node, NodeStrong};
use serde::export::Formatter;
use crate::gui::tree;
use crate::gui::event::{Priority, SharedGuiEvent};

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum PaintState {
    Normal,
    Over,
    Down,
}

pub struct Button<T: ButtonPaint = TextButtonPaint> {
    event: SharedGuiEvent,
    over: bool,
    down: bool,
    countdown: usize,
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
    pub fn new_from_paint(id: NodeStrong<Button<T>>, paint: T, event: SharedGuiEvent) -> View<Self> {
        View::new(id, Button {
            event,
            over: false,
            down: false,
            countdown: 0,
            state: PaintState::Normal,
            paint,
        })
    }
}

impl Button {
    pub fn new(id: NodeStrong<Button>, text: String, event: SharedGuiEvent) -> View<Button> {
        Button::new_from_paint(id, TextButtonPaint::new(text), event)
    }
}

impl<T: ButtonPaint> Button<T> {
    pub fn state(&self) -> PaintState {
        self.state
    }
    fn sync(self: &mut View<Self>) {
        let new_state =
            if self.down || self.countdown > 0 {
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

pub trait ButtonPaint: 'static + Sized + Debug + Send + Sync {
    fn button_paint(self: &Button<Self>, canvas: Canvas);
    fn button_layout(self: &mut Button<Self>, constraint: &Constraint) -> Layout;
}

impl ButtonPaint for TextButtonPaint {
    fn button_paint(self: &Button<Self>, mut w: Canvas) {
        w.style = match self.state() {
            PaintState::Normal => self.normal_style,
            PaintState::Over => self.over_style,
            PaintState::Down => self.down_style,
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

    fn button_layout(self: &mut Button<Self>, constraint: &Constraint) -> Layout {
        Layout {
            size: ((self.text.len() + 2) as isize, 3),
            line_settings: HashMap::new(),
        }
    }
}


impl<T: ButtonPaint> ViewImpl for Button<T> {
    fn self_handle(self: &mut View<Self>, event: &InputEvent) -> bool {
        match event {
            InputEvent::MouseEvent { event, inside } => {
                let was_down = self.down;
                let was_over = self.over;
                self.over = *inside;
                self.down = self.over && event.mouse == Mouse::Down(0);
                if was_down && !self.down && *inside {
                    self.event_sender().run(Priority::Now, self.event.once());
                    self.countdown += 1;
                    self.event_sender().run_with_delay(
                        Duration::from_millis(50),
                        self.node().new_event(|this| {
                            this.countdown -= 1;
                            this.sync();
                        })).ignore();
                }
            }
        }
        self.sync();
        true
    }

    fn layout_impl(self: &mut View<Self>, constraint: &Constraint) -> Layout {
        self.button_layout(constraint)
    }

    fn self_paint_below(self: &View<Self>, canvas: Canvas) {
        self.button_paint(canvas)
    }
}

impl<T: ButtonPaint> Deref for Button<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.paint }
}

impl<T: ButtonPaint> DerefMut for Button<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.paint }
}

impl<T: ButtonPaint> Debug for Button<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Button").field("paint", &self.paint).finish()
    }
}