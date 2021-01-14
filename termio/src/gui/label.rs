use crate::string::StyleString;
use crate::gui::node::{Node, NodeImpl};
use crate::gui::layout::Constraint;
use crate::canvas::Canvas;
use crate::gui::gui::{InputEvent, OutputEvent};
use crate::input::{MouseEvent, Mouse};
use std::mem;

#[derive(Debug)]
pub struct Label {
    lines: Vec<StyleString>,
    bottom_scroll: isize,
}

impl Label {
    pub fn new() -> Node<Self> {
        Node::new(Label {
            lines: vec![],
            bottom_scroll: 0,
        })
    }

    pub fn sync(self: &mut Node<Self>, source: &Vec<StyleString>) {
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

impl NodeImpl for Label {
    fn paint(self: &Node<Self>, mut canvas: Canvas) {
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

    fn layout(self: &mut Node<Self>, constraint: &Constraint) {
        if let Some(size) = constraint.max_size {
            self.set_size(size);
        }
    }

    fn handle(self: &mut Node<Self>, event: &InputEvent, output: &mut Vec<OutputEvent>) {
        match event {
            InputEvent::MouseEvent { event, inside } => {
                if *inside {
                    match event.mouse {
                        Mouse::ScrollDown => {
                            if self.bottom_scroll < self.lines.len() as isize {
                                self.bottom_scroll += 1;
                                self.mark_dirty();
                            }
                        }
                        Mouse::ScrollUp => {
                            if self.bottom_scroll > self.size().1 {
                                self.bottom_scroll -= 1;
                                self.mark_dirty();
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}