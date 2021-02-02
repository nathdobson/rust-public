use crate::gui::div::{DivImpl, DivRc, Div};
use crate::gui::layout::{Constraint, Layout};
use crate::screen::LineSetting;
use crate::gui::tree::{Dirty, Tree};
use crate::canvas::Canvas;
use crate::gui::gui::InputEvent;
use unicode_segmentation::UnicodeSegmentation;
use std::mem;
use std::iter;
use crate::color::Color;
use crate::input::{Mouse, KeyEvent, Key};
use crate::Direction;

#[derive(Debug)]
pub struct Field {
    content: String,
    enabled: bool,
    cursor: isize,
}

impl Field {
    pub fn new(tree: Tree, content: String) -> DivRc<Self> {
        DivRc::new_cyclic(tree, |this| {
            Field {
                content,
                enabled: true,
                cursor: 0,
            }
        })
    }
    pub fn clear(self: &mut Div<Self>) {
        if !self.content.is_empty() {
            self.content.clear();
            self.cursor = 0;
            self.mark_dirty(Dirty::Paint);
        }
    }
    pub fn set_enabled(self: &mut Div<Self>, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.mark_dirty(Dirty::Paint);
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn content(&self) -> &str {
        &self.content
    }
    pub fn set_cursor(self: &mut Div<Self>, mut cursor: isize) {
        cursor = cursor.max(0).min(self.content.graphemes(true).count() as isize);
        if self.cursor != cursor {
            self.cursor = cursor;
            self.mark_dirty(Dirty::Paint);
        }
    }
}

impl DivImpl for Field {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        let max = constraint.max_size.unwrap();
        Layout { size: (max.0, 1), line_settings: iter::once((0, LineSetting::Normal)).collect() }
    }
    fn self_handle(self: &mut Div<Self>, event: &InputEvent) -> bool {
        match event {
            InputEvent::MouseEvent { event, inside } => {
                if *inside && !event.motion && event.mouse == Mouse::Down(0) {
                    self.set_cursor(event.position.0);
                    return true;
                }
            }
            InputEvent::KeyEvent(KeyEvent { modifier, key }) => {
                match *key {
                    Key::Arrow(arrow) => {
                        match arrow {
                            Direction::Right => {
                                self.set_cursor(self.cursor + 1);
                                return true;
                            }
                            Direction::Left => {
                                self.set_cursor(self.cursor - 1);
                                return true;
                            }
                            _ => {}
                        }
                    }
                    Key::Type('\r') => {}
                    Key::Type(c) => {
                        let offset: usize = self.content.graphemes(true).take(self.cursor as usize).map(|x| x.len()).sum();
                        self.cursor += 1;
                        self.content.insert(offset, c);
                        self.mark_dirty(Dirty::Paint);
                    }
                    Key::Func(_) => {}
                    Key::Delete => {
                        let offset: usize = self.content.graphemes(true).take(self.cursor as usize).map(|x| x.len()).sum();
                        if offset > 0 {
                            self.cursor -= 1;
                            self.content.remove(offset - 1);
                            self.mark_dirty(Dirty::Paint);
                        }
                    }
                    Key::ForwardDelete => {
                        let offset: usize = self.content.graphemes(true).take(self.cursor as usize).map(|x| x.len()).sum();
                        if offset < self.content.len() {
                            self.content.remove(offset);
                            self.mark_dirty(Dirty::Paint);
                        }
                    }
                }
            }
        }
        false
    }
    fn self_paint_below(self: &Div<Self>, mut canvas: Canvas) {
        for (index, grapheme) in self.content.graphemes(true).chain(iter::once(" ")).enumerate() {
            let mut canvas = canvas.push();
            canvas.style.foreground = Color::Gray24(0);
            canvas.style.background = Color::Gray24(23);
            if index == self.cursor as usize && self.enabled {
                mem::swap(&mut canvas.style.background, &mut canvas.style.foreground);
            }
            canvas.draw((index as isize, 0), &grapheme);
        }
    }
}