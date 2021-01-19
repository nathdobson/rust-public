use std::collections::HashMap;
use crate::screen::LineSetting;
use crate::gui::node::{Node};
use crate::gui::view::{View};
use util::grid::Grid;
use util::itertools::Itertools2;

pub struct Constraint {
    pub max_size: Option<(isize, isize)>,
}

pub struct Layout {
    pub size: (isize, isize),
    pub line_settings: HashMap::<isize, LineSetting>,
}

impl Constraint {
    pub fn from_max(max_size: (isize, isize)) -> Self {
        assert!(max_size.0 >= 0);
        assert!(max_size.1 >= 0);
        Constraint { max_size: Some(max_size) }
    }
    pub fn none() -> Self {
        Constraint { max_size: None }
    }
    pub fn flow_layout(&self, view: &mut View, xs: isize, ys: isize) -> (isize, isize) {
        let mut x = 0;
        let mut y = 0;
        let mut width = 0;
        let mut height = 0;
        let max_width = self.max_size.unwrap_or((isize::MAX, 0)).0;
        for child in view.children_mut().into_iterable().into_iter() {
            let child = child(view);
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
    pub fn table_layout(&self, view: &mut View, children: &Grid<Node>) -> Layout {
        let (cols, rows) = children.size();
        let mut widths = vec![0; cols as usize];
        let mut heights = vec![0; rows as usize];
        for (p, child) in children.iter() {
            let child = child.child_mut(view);
            child.layout(&Constraint { max_size: None });
            let size = child.size();
            widths[p.0 as usize] = widths[p.0 as usize].max(size.0);
            heights[p.1 as usize] = heights[p.1 as usize].max(size.1);
        }
        let xs: Vec<isize> = widths.iter().scan_full(0, |x, w| x + 1 + *w).collect();
        let ys: Vec<isize> = heights.iter().scan_full(0, |y, h| y + 1 + *h).collect();

        for (p, child) in children.iter() {
            child.child_mut(view).set_position((xs[p.0 as usize] + 1, ys[p.1 as usize] + 1));
        }
        let (x, y) = (*xs.last().unwrap(), *ys.last().unwrap());
        Layout { size: (x, y), line_settings: HashMap::new() }
    }
}
