use std::collections::HashMap;

use crate::gui::div::{Div, DivImpl, DivRc};
use crate::gui::layout::{Align, Constraint, Layout};
use crate::gui::tree::{Dirty, Tree};

#[derive(Debug, Clone)]
pub struct SelectDiv {
    pub div: DivRc,
    pub align: (Align, Align),
}

#[derive(Debug)]
pub struct Select {
    selected: Option<DivRc>,
    children: Vec<SelectDiv>,
}

impl Select {
    pub fn new(
        tree: Tree,
        mut selected: Option<DivRc>,
        mut children: Vec<SelectDiv>,
    ) -> DivRc<Self> {
        for child in children.iter_mut() {
            child.div.write().set_visible(false);
        }
        if let Some(selected) = selected.as_mut() {
            selected.write().set_visible(true);
        }
        let mut result = DivRc::new(tree, Select { selected, children });
        {
            let mut result = result.write();
            for child in result.children.clone().into_iter() {
                result.add(child.div.clone());
            }
        }
        result
    }
    pub fn select(self: &mut Div<Self>, mut selected: Option<DivRc>) {
        if self.selected != selected {
            if let Some(old) = self.selected.as_mut() {
                old.write().set_visible(false);
            }
            if let Some(new) = selected.as_mut() {
                new.write().set_visible(true);
            }
            self.selected = selected;
            self.mark_dirty(Dirty::Layout);
        }
    }
}

impl DivImpl for Select {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        let mut size = (0, 0);
        for child in self.children.iter_mut() {
            let mut div = child.div.write();
            div.layout(constraint);
            size.0 = size.0.max(div.size().0);
            size.1 = size.1.max(div.size().1);
        }
        for child in self.children.iter_mut() {
            let mut div = child.div.write();
            div.set_position_aligned((0, 0), child.align, size);
        }
        Layout {
            size,
            line_settings: HashMap::new(),
        }
    }
}
