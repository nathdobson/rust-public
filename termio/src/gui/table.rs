use util::grid::Grid;
use util::grid::cells_by_row;
use crate::gui::node::{Node, NodeId};
use std::collections::HashMap;
use crate::gui::layout::Constraint;
use std::cmp::max;
use crate::line::{TableBorder, Stroke};
use crate::canvas::Canvas;
use util::itertools::Itertools2;
use std::ops::{Deref, DerefMut};
use crate::gui::group::{GroupImpl, Group};
use std::fmt::Debug;

#[derive(Debug)]
pub struct Table<T: TableImpl> {
    border: TableBorder,
    inner: T,
}

impl<T: TableImpl> Table<T> {
    pub fn new(id: NodeId, inner: T) -> Node<Group<Self>> {
        Group::new(id, Table {
            border: TableBorder {
                xs: vec![],
                ys: vec![],
                horizontals: Grid::default(),
                verticals: Grid::default(),
            },
            inner,
        })
    }
}

impl<T: TableImpl> GroupImpl for Table<T> {
    fn children(this: &Node<Group<Self>>) -> Vec<&Node> {
        T::table_children(this).into_iter().map(|x| x.1).collect()
    }

    fn children_mut(this: &mut Node<Group<Self>>) -> Vec<&mut Node> {
        T::table_children_mut(this).into_iter().map(|x| x.1).collect()
    }

    fn group_layout_self(this: &mut Node<Group<Self>>, constraint: &Constraint) -> (isize, isize) {
        let mut children = T::table_children_mut(this);
        let (cols, rows) = children.size();
        let mut widths = vec![0; cols as usize];
        let mut heights = vec![0; rows as usize];
        for (p, child) in children.iter_mut() {
            child.layout(&Constraint { max_size: None });
            let size = child.size();
            widths[p.0 as usize] = max(widths[p.0 as usize], size.0);
            heights[p.1 as usize] = max(heights[p.1 as usize], size.1);
        }
        let xs: Vec<isize> = widths.iter().scan_full(0, |x, w| x + 1 + *w).collect();
        let ys: Vec<isize> = heights.iter().scan_full(0, |y, h| y + 1 + *h).collect();

        for (p, child) in children.iter_mut() {
            child.set_position((xs[p.0 as usize] + 1, ys[p.1 as usize] + 1));
        }
        let (x, y) = (*xs.last().unwrap(), *ys.last().unwrap());
        this.border = TableBorder {
            xs,
            ys,
            horizontals: Grid::new((cols, rows + 1), |_, _| Stroke::Double),
            verticals: Grid::new((cols + 1, rows), |x, y| if x <= 1 { Stroke::Double } else { Stroke::Blank }),
        };
        (x, y)
    }

    fn group_paint_below(this: &Node<Group<Self>>, canvas: Canvas) {
        this.border.paint_border(canvas);
    }
}

pub trait TableImpl: Sized + Debug + Send + Sync {
    fn table_children(this: &Node<Group<Table<Self>>>) -> Grid<&Node>;
    fn table_children_mut(this: &mut Node<Group<Table<Self>>>) -> Grid<&mut Node>;
}

impl<T: TableImpl> Deref for Table<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: TableImpl> DerefMut for Table<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}