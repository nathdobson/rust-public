use std::collections::HashMap;

use crate::gui::div::{Div, DivImpl, DivRc};
use crate::gui::layout::{Constraint, Layout};
use crate::gui::tree::Tree;

#[derive(Debug)]
pub struct Pad {
    size: (isize, isize),
}

impl Pad {
    pub fn horizontal(tree: Tree, w: isize) -> DivRc<Pad> { Self::rectangle(tree, (w, 0)) }
    pub fn vertical(tree: Tree, h: isize) -> DivRc<Pad> { Self::rectangle(tree, (0, h)) }
    pub fn rectangle(tree: Tree, size: (isize, isize)) -> DivRc<Pad> {
        DivRc::new(tree, Pad { size })
    }
}

impl DivImpl for Pad {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        Layout {
            size: self.size,
            line_settings: HashMap::new(),
        }
    }
}
