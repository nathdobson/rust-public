use crate::string::StyleString;
use crate::gui::layout::{Constraint, Layout};
use crate::canvas::Canvas;
use crate::gui::gui::{InputEvent};
use crate::input::{MouseEvent, Mouse};
use std::mem;
use std::collections::HashMap;
use crate::gui::div::{DivRc, DivImpl, Div};
use crate::gui::tree::Tree;

#[derive(Debug)]
pub struct Label {
    lines: Vec<StyleString>,
    bottom_scroll: isize,
}

impl Label {
    pub fn new(tree: Tree) -> DivRc<Label> {
        DivRc::new(tree, Label {
            lines: vec![],
            bottom_scroll: 0,
        })
    }

    pub fn sync(self: &mut Div<Self>, source: &Vec<StyleString>) {
        if self.lines.len() < source.len() {
            let len = self.lines.len();
            self.lines.extend_from_slice(&source[len..]);
            self.bottom_scroll = self.lines.len() as isize;
            self.mark_dirty();
        } else if source.is_empty() && !self.lines.is_empty() {
            self.lines.clear();
            self.mark_dirty();
        }
    }
}

impl DivImpl for Label {
    fn self_paint_below(self: &Div<Self>, mut canvas: Canvas) {
        let (width, height) = self.size();
        for y in 0..self.size().1 {
            if let Some(line) = self.lines.get((y + self.bottom_scroll - height) as usize) {
                canvas.draw((0, y as isize), line);
            }
        }
        if height < self.lines.len() as isize {
            let top = self.bottom_scroll - height;
            let middle = height;
            let bottom = self.lines.len() as isize - middle - top;
            let top = ((height - 1) as f32 * 8.0 * top as f32 / (self.lines.len() as f32)).ceil() as isize;
            let bottom = ((height - 1) as f32 * 8.0 * bottom as f32 / (self.lines.len() as f32)).ceil() as isize;
            let middle = height * 8 - top - bottom;
            //println!("{:?} {:?} {:?}", top, middle, bottom);
            for y in 0..height {
                if y < top / 8 {
                    canvas.draw((width - 1, y), &' ');
                } else if y == top / 8 {
                    canvas.draw((width - 1, y),
                                &std::char::from_u32('█' as u32 - (top % 8) as u32).unwrap());
                } else if y < (top + middle) / 8 {
                    canvas.draw((width - 1, y), &'█');
                } else if y == (top + middle) / 8 {
                    let mut canvas2 = canvas.push();
                    mem::swap(&mut canvas2.style.background, &mut canvas2.style.foreground);
                    canvas2.draw((width - 1, y),
                                 &std::char::from_u32('█' as u32 - ((top + middle) % 8) as u32).unwrap());
                } else {
                    canvas.draw((width - 1, y), &' ');
                }
            }
        }
    }

    fn self_handle(self: &mut Div<Self>, event: &InputEvent) -> bool {
        match event {
            InputEvent::MouseEvent { event, inside } => {
                if *inside {
                    match event.mouse {
                        Mouse::ScrollDown => {
                            if self.bottom_scroll < self.lines.len() as isize {
                                self.bottom_scroll += 1;
                                self.mark_dirty();
                                return true;
                            }
                        }
                        Mouse::ScrollUp => {
                            if self.bottom_scroll > self.size().1 {
                                self.bottom_scroll -= 1;
                                self.mark_dirty();
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        false
    }

    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        Layout {
            size: constraint.max_size.unwrap_or(self.size()),
            line_settings: HashMap::new(),
        }
    }
}