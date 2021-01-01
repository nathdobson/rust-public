use std::ops::Deref;
use itertools::{Itertools, repeat_n};
use std::collections::{HashMap, BTreeMap};
use std::{iter, mem};
use std::fmt;
use crate::canvas::Canvas;
use util::grid::Grid;
use crate::string::StyleOption;
use crate::screen::Screen;
use crate::screen::Style;
use util::rect::Rect;

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Copy, Clone)]
pub enum Stroke {
    Blank,
    Narrow,
    Narrow2,
    Narrow3,
    Narrow4,
    Wide,
    Wide2,
    Wide3,
    Wide4,
    Double,
    Curved,
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct BoxCell {
    left: Stroke,
    right: Stroke,
    up: Stroke,
    down: Stroke,
}

impl BoxCell {
    fn new(left: Stroke, right: Stroke, up: Stroke, down: Stroke) -> Self {
        BoxCell { left, right, up, down }
    }
}

fn unrender(c: char) -> Option<BoxCell> {
    use Stroke::Blank as B;
    use Stroke::Narrow as N;
    use Stroke::Narrow2 as N2;
    use Stroke::Narrow3 as N3;
    use Stroke::Narrow4 as N4;
    use Stroke::Wide as T;
    use Stroke::Wide2 as T2;
    use Stroke::Wide3 as T3;
    use Stroke::Wide4 as T4;
    use Stroke::Double as D;
    use Stroke::Curved as C;

    let (left, right, up, down) = match c {
        '─' => (N, N, B, B),
        '━' => (T, T, B, B),
        '│' => (B, B, N, N),
        '┃' => (B, B, T, T),
        '┄' => (N3, N3, B, B),
        '┅' => (T3, T3, B, B),
        '┆' => (B, B, N3, N3),
        '┇' => (B, B, T3, T3),
        '┈' => (N4, N4, B, B),
        '┉' => (T4, T4, B, B),
        '┊' => (B, B, N4, N4),
        '┋' => (B, B, T4, T4),
        '┌' => (B, N, B, N),
        '┍' => (B, T, B, N),
        '┎' => (B, N, B, T),
        '┏' => (B, T, B, T),
        '┐' => (N, B, B, N),
        '┑' => (T, B, B, N),
        '┒' => (N, B, B, T),
        '┓' => (T, B, B, T),
        '└' => (B, N, N, B),
        '┕' => (B, T, N, B),
        '┖' => (B, N, T, B),
        '┗' => (B, T, T, B),
        '┘' => (N, B, N, B),
        '┙' => (T, B, N, B),
        '┚' => (N, B, T, B),
        '┛' => (T, B, T, B),
        '├' => (B, N, N, N),
        '┝' => (B, T, N, N),
        '┞' => (B, N, T, N),
        '┟' => (B, N, N, T),
        '┠' => (B, N, T, T),
        '┡' => (B, T, T, N),
        '┢' => (B, T, N, T),
        '┣' => (B, T, T, T),
        '┤' => (N, B, N, N),
        '┥' => (T, B, N, N),
        '┦' => (N, B, T, N),
        '┧' => (N, B, N, T),
        '┨' => (N, B, T, T),
        '┩' => (T, B, T, N),
        '┪' => (T, B, N, T),
        '┫' => (T, B, T, T),
        '┬' => (N, N, B, N),
        '┭' => (T, N, B, N),
        '┮' => (N, T, B, N),
        '┯' => (T, T, B, N),
        '┰' => (N, N, B, T),
        '┱' => (T, N, B, T),
        '┲' => (N, T, B, T),
        '┳' => (T, T, B, T),
        '┴' => (N, N, N, B),
        '┵' => (T, N, N, B),
        '┶' => (N, T, N, B),
        '┷' => (T, T, N, B),
        '┸' => (N, N, T, B),
        '┹' => (T, N, T, B),
        '┺' => (N, T, T, B),
        '┻' => (T, T, T, B),
        '┼' => (N, N, N, N),
        '┽' => (T, N, N, N),
        '┾' => (N, T, N, N),
        '┿' => (T, T, N, N),
        '╀' => (N, N, T, N),
        '╁' => (N, N, N, T),
        '╂' => (N, N, T, T),
        '╃' => (T, N, T, N),
        '╄' => (N, T, T, N),
        '╅' => (T, N, N, T),
        '╆' => (N, T, N, T),
        '╇' => (T, T, T, N),
        '╈' => (T, T, N, T),
        '╉' => (T, N, T, T),
        '╊' => (N, T, T, T),
        '╋' => (T, T, T, T),
        '╌' => (N2, N2, B, B),
        '╍' => (T2, T2, B, B),
        '╎' => (B, B, N2, N2),
        '╏' => (B, B, T2, T2),
        '═' => (D, D, B, B),
        '║' => (B, B, D, D),
        '╒' => (B, D, B, N),
        '╓' => (B, N, B, D),
        '╔' => (B, D, B, D),
        '╕' => (D, B, B, N),
        '╖' => (N, B, B, D),
        '╗' => (D, B, B, D),
        '╘' => (B, D, N, B),
        '╙' => (B, N, D, B),
        '╚' => (B, D, D, B),
        '╛' => (D, B, N, B),
        '╜' => (N, B, D, B),
        '╝' => (D, B, D, B),
        '╞' => (B, D, N, N),
        '╟' => (B, N, D, D),
        '╠' => (B, D, D, D),
        '╡' => (D, B, N, N),
        '╢' => (N, B, D, D),
        '╣' => (D, B, D, D),
        '╤' => (D, D, B, N),
        '╥' => (N, N, B, D),
        '╦' => (D, D, B, D),
        '╧' => (D, D, N, B),
        '╨' => (N, N, D, B),
        '╩' => (D, D, D, B),
        '╪' => (D, D, N, N),
        '╫' => (N, N, D, D),
        '╬' => (D, D, D, D),
        '╭' => (B, C, B, C),
        '╮' => (C, B, B, C),
        '╯' => (C, B, C, B),
        '╰' => (B, C, C, B),
        '╱' => return None,
        '╲' => return None,
        '╳' => return None,
        '╴' => (N, B, B, B),
        '╵' => (B, B, N, B),
        '╶' => (B, N, B, B),
        '╷' => (B, B, B, N),
        '╸' => (T, B, B, B),
        '╹' => (B, B, T, B),
        '╺' => (B, T, B, B),
        '╻' => (B, B, B, T),
        '╼' => (N, T, B, B),
        '╽' => (B, B, N, T),
        '╾' => (T, N, B, B),
        '╿' => (B, B, T, N),
        _ => return None,
    };
    Some(BoxCell { left, right, up, down })
}

fn build_table() -> HashMap<BoxCell, char> {
    (('─' as u32)..=('╿' as u32)).flat_map(
        |c| {
            let c = std::char::from_u32(c).unwrap();
            Some((unrender(c)?, c))
        }
    ).chain(iter::once((
        BoxCell::new(Stroke::Blank, Stroke::Blank, Stroke::Blank, Stroke::Blank), ' '))).collect()
}

lazy_static!(
    static ref LOOKUP : HashMap<BoxCell, char> = build_table();
);

impl fmt::Display for BoxCell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", LOOKUP.get(self).cloned().unwrap_or('?'))?;
        Ok(())
    }
}

#[test]
fn test_symbols() {
    let mut mapping = BTreeMap::<(&str, Stroke), Vec<char>>::new();
    for (&cell, &ch) in LOOKUP.iter() {
        mapping.entry(("left ", cell.left)).or_default().push(ch);
        mapping.entry(("right ", cell.right)).or_default().push(ch);
        mapping.entry(("up ", cell.up)).or_default().push(ch);
        mapping.entry(("down ", cell.down)).or_default().push(ch);
    }
    for xs in repeat_n([Stroke::Blank, Stroke::Narrow, Stroke::Wide, Stroke::Double].iter().cloned(), 4).multi_cartesian_product() {
        if xs.iter().any(|&x| x == Stroke::Wide) && xs.iter().any(|&x| x == Stroke::Double) {
            continue;
        }
        let (left, right, up, down) = (xs[0], xs[1], xs[2], xs[3]);
        if left == Stroke::Double && right != Stroke::Double {
            continue;
        }
        if left != Stroke::Double && right == Stroke::Double {
            continue;
        }
        if up == Stroke::Double && down != Stroke::Double {
            continue;
        }
        if up != Stroke::Double && down == Stroke::Double {
            continue;
        }
        println!(" {} ", BoxCell::new(Stroke::Blank, Stroke::Blank, up, up));
        println!("{}{}{}",
                 BoxCell::new(left, left, Stroke::Blank, Stroke::Blank),
                 BoxCell::new(left, right, up, down),
                 BoxCell::new(right, right, Stroke::Blank, Stroke::Blank));
        println!(" {} ", BoxCell::new(Stroke::Blank, Stroke::Blank, down, down));
        println!();
    }
}

#[derive(Debug, Default)]
pub struct TableBorder {
    pub xs: Vec<isize>,
    pub ys: Vec<isize>,
    pub horizontals: Grid<Stroke>,
    pub verticals: Grid<Stroke>,
}

impl TableBorder {
    pub fn paint_border(&self, mut canvas: Canvas) {
        for (row, y) in self.ys.iter().cloned().enumerate() {
            for (col, x) in self.xs.iter().cloned().enumerate() {
                let row = row as isize;
                let col = col as isize;
                let cross = BoxCell {
                    left: self.horizontals.get((col - 1, row)).cloned().unwrap_or(Stroke::Blank),
                    right: self.horizontals.get((col, row)).cloned().unwrap_or(Stroke::Blank),
                    up: self.verticals.get((col, row - 1)).cloned().unwrap_or(Stroke::Blank),
                    down: self.verticals.get((col, row)).cloned().unwrap_or(Stroke::Blank),
                };
                canvas.set((x, y), &format!("{}", cross));
            }
        }
        for (row, y) in self.ys.iter().cloned().enumerate() {
            for (col, xs) in self.xs.windows(2).enumerate() {
                for x in xs[0] + 1..xs[1] {
                    let col = col as isize;
                    let row = row as isize;
                    canvas.set((x, y), &format!("{}", BoxCell {
                        left: self.horizontals.get((col, row)).cloned().unwrap_or(Stroke::Blank),
                        right: self.horizontals.get((col, row)).cloned().unwrap_or(Stroke::Blank),
                        up: Stroke::Blank,
                        down: Stroke::Blank,
                    }));
                }
            }
        }
        for (row, ys) in self.ys.windows(2).enumerate() {
            for (col, x) in self.xs.iter().cloned().enumerate() {
                for y in ys[0] + 1..ys[1] {
                    let col = col as isize;
                    let row = row as isize;
                    canvas.set((x, y), &format!("{}", BoxCell {
                        left: Stroke::Blank,
                        right: Stroke::Blank,
                        up: self.verticals.get((col, row)).cloned().unwrap_or(Stroke::Blank),
                        down: self.verticals.get((col, row)).cloned().unwrap_or(Stroke::Blank),
                    }));
                }
            }
        }
    }
}

#[test]
fn test_table() {
    let mut screen = Screen::new();
    let canvas = Canvas::new(&mut screen,
                             Rect::from_position_size((0, 0), (20, 20)),
                             (0, 0),
                             Style::default());
    let table = TableBorder {
        xs: vec![0, 5, 10],
        ys: vec![3, 8, 13],
        horizontals: Grid::new((2, 3), |x, y| { Stroke::Narrow }),
        verticals: Grid::new((3, 2), |x, y| { Stroke::Narrow }),
    };
    println!("{:?}", table);
    table.paint_border(canvas);
    println!("{:?}", screen);
}