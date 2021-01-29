use crate::gui::event::SharedGuiEvent;
use crate::string::StyleString;
use crate::gui::div::{DivRc, DivImpl, Div};
use crate::gui::layout::{Constraint, Layout};
use crate::gui::tree::{Tree, Dirty};
use crate::advance::{advance_of_string, advance_of_style_string};
use std::collections::HashMap;
use crate::canvas::Canvas;
use crate::gui::gui::InputEvent;
use crate::input::Mouse;
use crate::color::Color;

#[derive(Debug)]
pub struct CheckBox {
    event: SharedGuiEvent,
    text: StyleString,
    state: bool,
}

impl CheckBox {
    pub fn new(tree: Tree, text: StyleString, state: bool, event: SharedGuiEvent) -> DivRc<CheckBox> {
        DivRc::new(tree, CheckBox { event, text, state })
    }
    pub fn get_state(&self) -> bool {
        self.state
    }
    pub fn set_state(self: &mut Div<Self>, state: bool) {
        if self.state != state {
            self.state = state;
            self.mark_dirty(Dirty::Paint);
        }
    }
}

impl DivImpl for CheckBox {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        Layout {
            size: ((1 + advance_of_style_string(&self.text)) as isize, 1),
            line_settings: HashMap::new(),
        }
    }
    fn self_paint_below(self: &Div<Self>, mut canvas: Canvas) {
        if self.state {
            let mut canvas = canvas.push();
            canvas.style.background = Color::RGB666(0, 1, 4);
            canvas.style.foreground=Color::Gray24(23);
            canvas.draw((0, 0), &"☑");
        } else {
            canvas.draw((0, 0), &"☐");
        }
        canvas.draw((1, 0), &self.text);
    }
    fn self_handle(self: &mut Div<Self>, event: &InputEvent) -> bool {
        match event {
            InputEvent::MouseEvent { event, inside } => {
                if *inside && !event.motion && event.mouse == Mouse::Down(0) {
                    self.state = !self.state;
                    self.event_sender().run_now(self.event.once());
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}

