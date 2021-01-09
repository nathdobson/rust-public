use std::collections::HashMap;
use crate::screen::LineSetting;
use crate::gui::node::Node;

pub struct Constraint {
    pub max_size: Option<(isize, isize)>,
}

impl Constraint {
    pub fn from_max(max_size: (isize, isize)) -> Self {
        Constraint { max_size: Some(max_size) }
    }
    pub fn none() -> Self {
        Constraint { max_size: None }
    }
    pub fn flow_layout<'a>(&'a self, children: impl Iterator<Item=&'a mut Node>, xs: isize, ys: isize) -> (isize, isize) {
        let mut x = 0;
        let mut y = 0;
        let mut width = 0;
        let mut height = 0;
        let max_width = self.max_size.unwrap_or((isize::MAX, 0)).0;
        for child in children {
            child.layout(&Constraint::none());
            if x + child.size().0 > max_width {
                x = 0;
                y = height + ys;
            }
            child.set_position((x, y));
            x += child.size().1 + xs;
            width = width.max(x + child.size().0);
            height = height.max(y + child.size().1);
        }
        (width, height)
    }
}
