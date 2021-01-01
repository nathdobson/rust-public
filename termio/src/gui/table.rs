use util::grid::Grid;
use util::grid::cells_by_row;
use crate::gui::container::ContainerImpl;
use crate::gui::node::Node;
use std::collections::HashMap;
use crate::gui::layout::Constraint;
use std::cmp::max;
use crate::line::{TableBorder, Stroke};
use crate::canvas::Canvas;
use util::itertools::Itertools2;

pub struct Table<T: TableImpl> {
    border: TableBorder,
    inner: T,
}

impl<T: TableImpl> Table<T> {
    pub fn new(inner: T) -> Self {
        Table {
            border: TableBorder {
                xs: vec![],
                ys: vec![],
                horizontals: Grid::default(),
                verticals: Grid::default(),
            },
            inner,
        }
    }
}

impl<T: TableImpl> ContainerImpl for Table<T> {
    fn children(&self) -> Vec<&dyn Node> {
        self.inner.children().into_iter().map(|x| x.1).collect()
    }

    fn children_mut(&mut self) -> Vec<&mut dyn Node> {
        self.inner.children_mut().into_iter().map(|x| x.1).collect()
    }

    fn layout(&mut self, constraint: &Constraint) -> (isize, isize) {
        let mut children = self.inner.children_mut();
        let (cols, rows) = children.size();
        let mut widths = vec![0; cols as usize];
        let mut heights = vec![0; rows as usize];
        for (p, child) in children.iter_mut() {
            child.layout(&Constraint { max_size: None });
            let size = child.header().size();
            widths[p.0 as usize] = max(widths[p.0 as usize], size.0);
            heights[p.1 as usize] = max(heights[p.1 as usize], size.1);
        }
        let xs: Vec<isize> = widths.iter().scan_full(0, |x, w| x + 1 + *w).collect();
        let ys: Vec<isize> = heights.iter().scan_full(0, |y, h| y + 1 + *h).collect();

        for (p, child) in children.iter_mut() {
            child.header_mut().set_position((xs[p.0 as usize] + 1, ys[p.1 as usize] + 1));
        }
        let (x, y) = (*xs.last().unwrap(), *ys.last().unwrap());
        self.border = TableBorder {
            xs,
            ys,
            horizontals: Grid::new((cols, rows + 1), |_, _| Stroke::Double),
            verticals: Grid::new((cols + 1, rows), |_, _| Stroke::Double),
        };
        (x, y)
    }
    fn paint(&self, canvas: Canvas) {
        self.border.paint_border(canvas);
    }
}

pub trait TableImpl {
    fn children(&self) -> Grid<&dyn Node>;
    fn children_mut(&mut self) -> Grid<&mut dyn Node>;
}
