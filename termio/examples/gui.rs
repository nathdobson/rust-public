#![feature(never_type, arbitrary_self_types)]
#![feature(box_syntax)]
#![allow(unused_imports)]

use termio::gui::gui::{Gui, InputEvent};
use termio::gui::button::Button;
use std::sync::{Arc, mpsc, Mutex};
use termio::input::{EventReader, Event, KeyEvent};
use std::io::{stdin, stdout, Write};
use std::error::Error;
use std::{mem, thread, process};
use util::grid::Grid;
use termio::gui::label::Label;
use termio::string::{StyleFormatExt, StyleString};
use util::any::{Upcast, AnyExt};
use std::any::Any;
use std::ops::Deref;
use termio::screen::Style;
use termio::color::Color;
use timer::Timer;
use std::time::Instant;
use chrono;
use std::time;
use termio::gui::tree::{Tree};
use termio::gui::layout::{Constraint, Layout};
use util::lossy;
use std::str;
use termio::gui::event::{Priority, GuiEvent, SharedGuiEvent};
use termio::gui::event;
use termio::gui::div::{Div, DivImpl, DivRc, DivWeak};
use util::atomic_refcell::AtomicRefCell;
use util::mutrc::MutRc;

#[derive(Debug)]
struct Example {
    model: Vec<StyleString>,
    buttons: Vec<DivRc<Button>>,
    labels: Vec<DivRc<Label>>,
    grid: Grid<DivRc>,
}

impl Example {
    fn new(tree: Tree) -> DivRc<Self> {
        let mut result = DivRc::new_cyclic(tree.clone(), |example: DivWeak<Example>| {
            let model = vec!["a".to_style_string()];
            let mut labels = vec![Label::new(tree.clone()), Label::new(tree.clone())];
            labels[0].write().sync(&model);
            labels[0].write().set_size((40, 10));
            labels[1].write().set_size((10, 10));
            let buttons: Vec<DivRc<Button>> =
                ["a", "bb", "ccc", "dddd"].iter()
                    .map(|s| {
                        let ss = s.to_style_string();
                        Button::new(
                            tree.clone(),
                            s.to_string(),
                            example.new_shared_event(move |e| {
                                let e = &mut **e;
                                e.model.push(ss.clone());
                                e.labels[0].write().sync(&e.model);
                            }),
                        )
                    }).collect();
            let grid = Grid::new((2, 3), |x, y| {
                match (x, y) {
                    (0, 0) => buttons[0].clone().upcast_div(),
                    (1, 0) => buttons[1].clone(),
                    (0, 1) => buttons[2].clone(),
                    (1, 1) => buttons[3].clone(),
                    (0, 2) => labels[0].clone(),
                    (1, 2) => labels[1].clone(),
                    _ => panic!()
                }
            });
            Example {
                model,
                buttons,
                labels,
                grid,
            }
        });
        let mut this = result.write();
        for button in this.buttons.clone().iter() {
            this.add(button.clone())
        }
        for label in this.labels.clone().iter() {
            this.add(label.clone());
        }
        mem::drop(this);
        result
    }
    fn new_gui(tree: Tree) -> MutRc<Gui> {
        let mut gui = Gui::new(tree.clone(), Example::new(tree));
        gui.set_background(Style {
            background: Color::Gray24(23),
            foreground: Color::Gray24(0),
            ..Style::default()
        });
        MutRc::new(gui)
    }
}

impl DivImpl for Example {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        constraint.table_layout(&mut self.grid)
    }
    fn self_handle(self: &mut Div<Self>, event: &InputEvent) -> bool {
        if *event == InputEvent::KeyEvent(KeyEvent::typed('c').control()) {
            eprintln!("Quitting");
            std::process::exit(0);
        }
        false
    }
}

fn main() {
    event::run_local(|tree| Example::new_gui(tree));
}
