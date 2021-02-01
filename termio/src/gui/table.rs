use crate::gui::tree::Tree;
use util::grid::{Grid, rows, cols};
use crate::gui::div::{DivRc, DivImpl, Div};
use crate::gui::layout::{Layout, Constraint};
use std::collections::HashMap;
use float_ord::FloatOrd;
use crate::line::{Stroke, TableBorder};
use crate::canvas::Canvas;

#[derive(Debug, Clone)]
pub struct TableDiv {
    pub div: DivRc,
    pub flex: bool,
    pub align: (f64, f64),
}

#[derive(Debug)]
pub struct Table {
    grid: Grid<TableDiv>,
    cols: Vec<f64>,
    rows: Vec<f64>,
    border: TableBorder,
}

impl Table {
    pub fn new(
        tree: Tree,
        grid: Grid<TableDiv>,
        cols: Vec<f64>,
        rows: Vec<f64>,
        horizontals: Grid<Stroke>,
        verticals: Grid<Stroke>,
    ) -> DivRc<Table> {
        let mut result = DivRc::new(tree, Table {
            grid,
            cols,
            rows,
            border: TableBorder {
                xs: vec![],
                ys: vec![],
                horizontals,
                verticals,
            },
        });
        {
            let mut write = result.write();
            let write = &mut *write;
            for child in write.grid.values().cloned().collect::<Vec<_>>().iter() {
                write.add(child.div.clone())
            }
        }
        result
    }
}

// Increase some values in sizes so:
// * sum(sizes)==total_size
// * modified values are proportional to portions
// * unmodified values are larger that proportional to portions
pub(crate) fn flex(total_size: isize, sizes: &mut [isize], portions: &[f64]) {
    assert_eq!(sizes.len(), portions.len());
    let mut flex_size: isize = total_size;
    let mut flex_portion: f64 = portions.iter().sum();
    if flex_portion == 0.0 {
        return;
    }
    let mut order = (0..portions.len()).collect::<Vec<_>>();
    order.sort_by_key(|&index| FloatOrd(portions[index] / sizes[index] as f64));
    let mut flex_start = 0;
    for &index in order.iter() {
        if flex_portion * sizes[index] as f64 <= portions[index] * flex_size as f64 && (portions[index] != 0.0 || sizes[index] != 0) {
            break;
        }
        flex_start += 1;
        flex_portion -= portions[index];
        flex_size -= sizes[index];
    }
    for &index in order[flex_start..].iter() {
        let new_size = ((flex_size as f64 / flex_portion) * portions[index]).round() as isize;
        flex_size -= new_size;
        flex_portion -= portions[index];
        sizes[index] = new_size;
    }
}

impl DivImpl for Table {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        let size = constraint.max_size.unwrap();
        let this = &mut **self;
        let mut widths = vec![0; this.grid.size().0 as usize];
        let mut heights = vec![0; this.grid.size().1 as usize];
        for ((x, y), cell) in this.grid.iter_mut() {
            let mut div = cell.div.write();
            if !cell.flex {
                div.layout(constraint);
                widths[x as usize] = widths[x as usize].max(div.size().0);
                heights[y as usize] = heights[y as usize].max(div.size().1);
            }
        }

        flex(size.0 - this.grid.size().0 - 1, &mut widths, &this.cols);
        flex(size.1 - this.grid.size().1 - 1, &mut heights, &this.rows);
        let mut xs = vec![0];
        for &width in widths.iter() {
            xs.push(xs.last().unwrap() + width + 1);
        }
        this.border.xs.clone_from(&xs);
        let mut ys = vec![0];
        for &height in heights.iter() {
            ys.push(ys.last().unwrap() + height + 1);
        }
        this.border.ys.clone_from(&ys);
        for y in 0..this.grid.size().1 {
            let height = heights[y as usize];
            for x in 0..this.grid.size().0 {
                let width = widths[x as usize];
                let cell = &mut this.grid[(x, y)];
                let mut div = cell.div.write();
                if cell.flex {
                    div.layout(&Constraint::from_max((width, height)));
                }
                let position =
                    (xs[x as usize] + 1 + (cell.align.0 * (width - div.size().0) as f64).round() as isize,
                     ys[y as usize] + 1 + (cell.align.1 * (height - div.size().1) as f64).round() as isize);
                div.set_position(position);
            }
        }
        Layout { size, line_settings: HashMap::new() }
    }

    fn self_paint_below(self: &Div<Self>, canvas: Canvas) {
        self.border.paint_border(canvas);
    }
}

#[cfg(test)]
mod test {
    use crate::gui::table::{flex};

    fn run_test(available: isize, sizes: &[isize], lines: &[f64], expected: &[isize]) {
        let mut sizes = sizes.iter().cloned().collect::<Vec<_>>();
        flex(available, &mut sizes, lines);
        assert_eq!(sizes, expected);
    }

    #[test]
    fn test() {
        run_test(0, &[], &[], &[]);
        run_test(1, &[], &[], &[]);
        run_test(0, &[0], &[0.0], &[0]);
        run_test(0, &[1], &[0.0], &[1]);
        run_test(1, &[1], &[0.0], &[1]);
        run_test(2, &[1], &[0.0], &[1]);
        run_test(0, &[0], &[1.0], &[0]);
        run_test(0, &[1], &[1.0], &[1]);
        run_test(1, &[1], &[1.0], &[1]);
        run_test(2, &[1], &[1.0], &[2]);
        run_test(3, &[1], &[1.0], &[3]);
        run_test(10, &[0, 0], &[1.0, 1.0], &[5, 5]);
        run_test(10, &[0, 0], &[1.0, 0.0], &[10, 0]);
        run_test(10, &[0, 0], &[0.0, 1.0], &[0, 10]);
        run_test(10, &[0, 0], &[0.0, 0.0], &[0, 0]);
        run_test(10, &[1, 1], &[1.0, 1.0], &[5, 5]);
        run_test(10, &[1, 1], &[1.0, 0.0], &[9, 1]);
        run_test(10, &[1, 1], &[0.0, 1.0], &[1, 9]);
    }

    #[test]
    fn test_all() {
        let max = 20;
        for a in 0..max {
            for s1 in 0..max {
                for s2 in 0..max - s1 {
                    for &p1 in &[0.0, 1.0, 2.0, 3.0] {
                        for &p2 in &[0.0, 1.0, 2.0, 3.0] {
                            let mut sizes = [s1, s2];
                            flex(a, &mut sizes, &[p1, p2]);
                            if p1 + p2 == 0.0 {
                                assert_eq!(sizes[0], s1);
                                assert_eq!(sizes[1], s2);
                            } else {
                                assert!(sizes[0] >= s1);
                                assert!(sizes[1] >= s2);
                                assert_eq!(sizes[0] + sizes[1], a.max(s1 + s2));
                            }
                            if sizes[0] != s1 && sizes[1] != s2 {
                                let ral = (sizes[0] as f64 - 1.0) / (sizes[1] as f64 + 1.0);
                                let rag = (sizes[0] as f64 + 1.0) / (sizes[1] as f64 - 1.0);
                                let re = p1 / p2;
                                assert!(ral <= re);
                                assert!(rag >= re);
                            }
                        }
                    }
                }
            }
        }
    }
}
