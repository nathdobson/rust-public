use std::collections::HashMap;
use std::mem;
use std::ops::{Index, IndexMut};

use crate::gui::div::{Div, DivImpl, DivRc, DivWeak};
use crate::gui::layout::{Align, Constraint, Layout, CENTER};
use crate::gui::pad::Pad;
use crate::gui::tree::Tree;

#[derive(Debug)]
pub enum FlowRule {
    Fill,
    Portion(f64),
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum FlowDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug)]
pub struct FlowDiv {
    pub div: DivRc,
    pub rule: FlowRule,
    pub align: (Align, Align),
}

#[derive(Debug)]
pub struct Flow {
    direction: FlowDirection,
    elements: Vec<FlowDiv>,
}

impl Flow {
    pub fn new(tree: Tree, direction: FlowDirection, elements: Vec<FlowDiv>) -> DivRc<Self> {
        let mut result = DivRc::new(
            tree,
            Flow {
                direction,
                elements,
            },
        );
        let mut write = result.write();
        for child in write
            .elements
            .iter()
            .map(|x| x.div.clone())
            .collect::<Vec<_>>()
            .into_iter()
        {
            write.add(child)
        }
        mem::drop(write);
        result
    }
}

impl FlowDiv {
    pub fn pad(tree: Tree, x: isize) -> Self {
        FlowDiv {
            div: Pad::rectangle(tree, (x, x)),
            rule: FlowRule::Fill,
            align: (CENTER, CENTER),
        }
    }
}

impl FlowDirection {
    fn flip(self) -> Self {
        match self {
            FlowDirection::Horizontal => FlowDirection::Vertical,
            FlowDirection::Vertical => FlowDirection::Horizontal,
        }
    }
}

impl<A> Index<FlowDirection> for (A, A) {
    type Output = A;
    fn index(&self, index: FlowDirection) -> &Self::Output {
        match index {
            FlowDirection::Horizontal => &self.0,
            FlowDirection::Vertical => &self.1,
        }
    }
}

impl<A> IndexMut<FlowDirection> for (A, A) {
    fn index_mut(&mut self, index: FlowDirection) -> &mut Self::Output {
        match index {
            FlowDirection::Horizontal => &mut self.0,
            FlowDirection::Vertical => &mut self.1,
        }
    }
}

impl DivImpl for Flow {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        let this = &mut **self;
        let d = this.direction;
        let f = d.flip();
        let size = constraint.max_size.unwrap();
        let mut actual_flip_size = 0;
        let mut available_portions = 0.0;
        let mut available = size[d];
        for child in this.elements.iter_mut() {
            match child.rule {
                FlowRule::Fill => {
                    let mut child = child.div.write();
                    child.layout(constraint);
                    available -= child.size()[d];
                    actual_flip_size = actual_flip_size.max(child.size()[f]);
                }
                FlowRule::Portion(portion) => {
                    available_portions += portion;
                }
            }
        }
        let mut position = 0;
        for child in this.elements.iter_mut() {
            let mut div = child.div.write();
            let delta;
            match child.rule {
                FlowRule::Fill => {
                    delta = div.size()[d];
                }
                FlowRule::Portion(portion) => {
                    delta = ((available as f64 * (portion / available_portions)).round() as isize)
                        .max(0);
                    available -= delta;
                    available_portions -= portion;
                    let mut max = size;
                    max[d] = delta;
                    div.layout(&Constraint::from_max(max));
                    actual_flip_size = actual_flip_size.max(div.size()[f]);
                }
            }
            let mut p = (0, 0);
            p[d] = child.align[d].align(position, div.size()[d], delta);
            div.set_position(p);
            position += delta;
        }
        for child in this.elements.iter_mut() {
            let mut div = child.div.write();
            let mut p = div.position();
            p[f] = child.align[f].align(0, div.size()[f], actual_flip_size);
            div.set_position(p);
        }
        let mut size = size;
        size[d] = position;
        size[f] = actual_flip_size;
        Layout {
            size,
            line_settings: HashMap::new(),
        }
    }
}
