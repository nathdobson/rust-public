use termio::gui::div::{DivImpl, Div, DivRc};
use termio::gui::layout::{Layout, Constraint};
use std::collections::HashMap;
use crate::puzzle::{Puzzle, Cell, Clue, Variable};
use termio::canvas::Canvas;
use std::ops::{Range, Bound};
use termio::gui::tree::{Tree, Dirty};
use util::rect::Rect;
use termio::color::Color;
use itertools::Itertools;
use termio::screen::Style;
use termio::gui::gui::InputEvent;
use termio::input::Key;
use termio::Direction;
use std::env::var;
use termio::input::KeyEvent;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Mode {
    Clue,
    Quote,
    Title,
}

#[derive(Debug)]
pub struct PuzzleDiv {
    puzzle: Puzzle,
    current_cell: usize,
    mode: Mode,
    lines: Vec<usize>,
}

impl PuzzleDiv {
    pub fn new(tree: Tree, puzzle: Puzzle) -> DivRc<Self> {
        DivRc::new(tree, PuzzleDiv {
            puzzle,
            current_cell: 0,
            mode: Mode::Clue,
            lines: vec![],
        })
    }
    fn current_variable(&self) -> Option<&Variable> {
        Some(&self.puzzle.variables[&self.puzzle.cells[self.current_cell].variable?])
    }
    fn set_variable(&mut self, variable: usize) {
        self.current_cell = self.puzzle.variables[&variable].cell;
    }
    fn current_clue(&self) -> Option<&Clue> {
        Some(&self.puzzle.clues[&self.current_variable()?.letter])
    }
    fn style(indexed: bool, active: bool) -> Style {
        Style {
            background:
            if indexed {
                if active {
                    Color::RGB666(4, 4, 0)
                } else {
                    Color::RGB666(4, 4, 3)
                }
            } else if active {
                Color::RGB666(2, 2, 5)
            } else {
                Color::Gray24(23)
            },
            foreground: Color::Gray24(0),
            ..Style::default()
        }
    }
    fn dark_style(indexed: bool, active: bool) -> Style {
        Style {
            background:
            if indexed {
                if active {
                    Color::RGB666(2, 2, 0)
                } else {
                    Color::RGB666(2, 2, 1)
                }
            } else {
                Color::Gray24(0)
            },
            foreground: Color::Gray24(0),
            ..Style::default()
        }
    }
    fn paint_clue(&self, mut canvas: Canvas) {
        if let Some(clue) = self.current_clue() {
            canvas.draw((0, 0), &clue.clue);
            for (x, v) in clue.variables.iter().cloned().enumerate() {
                let v = &self.puzzle.variables[&v];
                let cell = &self.puzzle.cells[v.cell];
                canvas.style = Self::style(self.current_variable() == Some(v), self.mode == Mode::Clue);
                canvas.draw((x as isize, 1), &cell.guess.unwrap_or(' '));
            }
        }
    }
    fn paint_quote(&self, mut canvas: Canvas) {
        for (y, (start, end)) in self.lines.iter().cloned().tuple_windows().enumerate() {
            for (x, cell) in (start..end).enumerate() {
                let cell: &Cell = &self.puzzle.cells[cell];
                let letter = cell.guess.unwrap_or(' ');
                canvas.style = Self::style(self.current_cell == cell.cell, self.mode == Mode::Quote);
                if cell.variable.is_none() && letter == ' ' {
                    canvas.style = Self::dark_style(self.current_cell == cell.cell, self.mode == Mode::Quote);
                }
                canvas.draw((x as isize, y as isize), &letter);
            }
        }
    }
    fn paint_title(&self, mut canvas: Canvas) {
        for (x, (letter, clue)) in self.puzzle.clues.iter().enumerate() {
            let cell = self.puzzle.variables[&clue.variables[0]].cell;
            let cell = &self.puzzle.cells[cell];
            canvas.style = Self::style(self.current_cell == cell.cell, self.mode == Mode::Title);
            canvas.draw((x as isize, 0 as isize), &cell.guess.unwrap_or(' '));
        }
    }
    fn wrap(x: usize, dir: isize, len: usize) -> usize {
        ((x as isize + dir as isize + len as isize) as usize) % len
    }
    fn shift_clue(&mut self, dir: isize) {
        let mut next_clue = self.puzzle.clues.first_key_value();
        if let Some(clue) = self.current_clue() {
            let letter = clue.letter;
            if dir == 1 {
                let range = (Bound::Excluded(letter), Bound::Unbounded);
                next_clue = self.puzzle.clues.range(range).next().or(self.puzzle.clues.first_key_value());
            } else if dir == -1 {
                let range = ..letter;
                next_clue = self.puzzle.clues.range(range).next_back().or(self.puzzle.clues.last_key_value());
            } else {
                panic!();
            };
        }
        let variable = next_clue.unwrap().1.variables[0];
        self.set_variable(variable);
    }
    fn shift(&mut self, dir: isize) {
        match self.mode {
            Mode::Clue => {
                if let Some(variable) = self.current_variable() {
                    let clue = self.current_clue().unwrap();
                    let offset = Self::wrap(variable.offset, dir, clue.variables.len());
                    let variable = clue.variables[offset];
                    self.set_variable(variable);
                } else {
                    self.current_cell = 0;
                }
            }
            Mode::Quote => {
                self.current_cell = Self::wrap(self.current_cell, dir, self.puzzle.cells.len());
            }
            Mode::Title => {
                self.shift_clue(dir);
            }
        }
    }
    fn shift_vertical(&mut self, dir: isize) {
        if self.mode == Mode::Quote {
            let y = self.lines.iter().position(|c| *c > self.current_cell).unwrap() - 1;
            let x = self.current_cell - self.lines[y];
            let y = Self::wrap(y, dir, self.lines.len() - 1);
            self.current_cell = x + self.lines[y];
        }
    }
}

impl DivImpl for PuzzleDiv {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        self.lines = (0..6).map(|x| x * self.puzzle.cells.len() / 5).collect();
        Layout { size: constraint.max_size.unwrap(), line_settings: HashMap::new() }
    }
    fn self_paint_below(self: &Div<Self>, mut canvas: Canvas) {
        self.paint_clue(canvas.push_bounds(Rect::from_position_size((0, 0), (self.size().0, 2))));
        self.paint_quote(canvas.push_translate((0, 4)).push_bounds(Rect::from_position_size((0, 4), (self.size().0, self.lines.len() as isize))));
        self.paint_title(canvas
            .push_translate((0, (4 + self.lines.len() + 1) as isize))
            .push_bounds(Rect::from_position_size((0, (4 + self.lines.len() + 1) as isize), (self.size().0, 1))));
    }
    fn self_handle(self: &mut Div<Self>, event: &InputEvent) -> bool {
        let this = &mut **self;
        match event {
            InputEvent::MouseEvent { event, inside } => {}
            InputEvent::KeyEvent(event) => {
                if *event == KeyEvent::typed('¡') {
                    this.mode = Mode::Clue;
                    while this.current_variable().is_none() {
                        this.current_cell = Self::wrap(this.current_cell, 1, this.puzzle.cells.len());
                    }
                    self.mark_dirty(Dirty::Paint);
                    return true;
                } else if *event == KeyEvent::typed('™') {
                    this.mode = Mode::Quote;
                    self.mark_dirty(Dirty::Paint);
                    return true;
                } else if *event == KeyEvent::typed('£') {
                    loop {
                        if let Some(variable) = this.current_variable() {
                            let variable = this.puzzle.clues[&variable.letter].variables[0];
                            this.set_variable(variable);
                            break;
                        } else {
                            this.current_cell = Self::wrap(this.current_cell, 1, this.puzzle.cells.len());
                        }
                    }
                    self.mode = Mode::Title;
                    self.mark_dirty(Dirty::Paint);
                    return true;
                } else if *event == KeyEvent::typed('\r') {
                    match this.mode {
                        Mode::Clue => {
                            self.shift_clue(1);
                        }
                        Mode::Quote => {
                            self.current_cell = self.puzzle.cells[self.current_cell + 1..].iter()
                                .chain(self.puzzle.cells[..self.current_cell + 1].iter())
                                .position(|x| x.variable.is_none())
                                .unwrap() + 1;
                        }
                        Mode::Title => {
                            self.shift_clue(1);
                        }
                    }
                    self.mark_dirty(Dirty::Paint);
                    return true;
                } else {
                    match event.key {
                        Key::Arrow(direction) => {
                            match direction {
                                Direction::Up => self.shift_vertical(-1),
                                Direction::Down => self.shift_vertical(1),
                                Direction::Right => self.shift(1),
                                Direction::Left => self.shift(-1),
                            }
                            self.mark_dirty(Dirty::Paint);
                            return true;
                        }
                        Key::Type(x) => {
                            if x.is_ascii_alphanumeric() {
                                if this.current_variable().is_some() {
                                    this.puzzle.cells[this.current_cell].guess = Some(x);
                                }
                                self.shift(1);
                                self.mark_dirty(Dirty::Paint);
                                return true;
                            }
                        }
                        Key::Func(_) => {}
                        Key::Delete => {
                            this.shift(-1);
                            if this.current_variable().is_some() {
                                this.puzzle.cells[this.current_cell].guess = None;
                            }
                            self.mark_dirty(Dirty::Paint);
                            return true;
                        }
                        Key::ForwardDelete => {}
                    }
                }
            }
        }
        false
    }
}

