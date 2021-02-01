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
use std::any::Any;
use std::ops::Deref;
use termio::screen::{Style, Rune};
use termio::color::Color;
use std::time::Instant;
use chrono;
use std::time;
use termio::gui::tree::{Tree};
use termio::gui::layout::{Constraint, Layout};
use util::lossy;
use std::str;
use termio::gui::event::{GuiEvent, SharedGuiEvent};
use termio::gui::event;
use termio::gui::div::{Div, DivImpl, DivRc, DivWeak};
use util::atomic_refcell::AtomicRefCell;
use util::mutrc::MutRc;
use futures::executor::block_on;
use termio::gui::run_local;
use termio::gui::checkbox::CheckBox;
use termio::gui::table::{Table, TableDiv};
use termio::line::Stroke;
use termio::canvas::Canvas;

#[derive(Debug)]
struct Example {
    model: Vec<StyleString>,
    buttons: Vec<DivRc<Button>>,
    checkboxes: Vec<DivRc<CheckBox>>,
    labels: Vec<DivRc<Label>>,
    table: DivRc<Table>,
}

impl Example {
    fn new(tree: Tree) -> DivRc<Self> {
        let mut result = DivRc::new_cyclic(tree.clone(), |example: DivWeak<Example>| {
            let model = vec!["a".to_style_string()];
            let mut labels = vec![Label::new(tree.clone()), Label::new(tree.clone())];
            labels[0].write().sync_log(&model);
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
                                e.labels[0].write().sync_log(&e.model);
                            }),
                        )
                    }).collect();
            let checkboxes: Vec<DivRc<CheckBox>> = ["Red", "Green"].iter().map(|s| {
                let ss = s.to_style_string();
                CheckBox::new(tree.clone(), ss.clone(), false, example.new_shared_event(move |e| {
                    let e = &mut **e;
                    e.model.push(ss.clone());
                    e.labels[0].write().sync_log(&e.model);
                }))
            }).collect();
            let grid = Grid::new((2, 4), |x, y| {
                match (x, y) {
                    (0, 0) => TableDiv {
                        div: buttons[0].clone(),
                        flex: false,
                        align: (0.0, 0.0),
                    },
                    (1, 0) => TableDiv {
                        div: buttons[1].clone(),
                        flex: false,
                        align: (0.0, 0.0),
                    },
                    (0, 1) => TableDiv {
                        div: buttons[2].clone(),
                        flex: false,
                        align: (0.0, 0.0),
                    },
                    (1, 1) => TableDiv {
                        div: buttons[3].clone(),
                        flex: false,
                        align: (0.0, 0.0),
                    },
                    (0, 2) => TableDiv {
                        div: checkboxes[0].clone(),
                        flex: false,
                        align: (0.0, 0.0),
                    },
                    (1, 2) => TableDiv {
                        div: checkboxes[1].clone(),
                        flex: false,
                        align: (0.0, 0.0),
                    },
                    (0, 3) => TableDiv {
                        div: labels[0].clone(),
                        flex: true,
                        align: (0.0, 0.0),
                    },
                    (1, 3) => TableDiv {
                        div: labels[1].clone(),
                        flex: true,
                        align: (0.0, 0.0),
                    },
                    _ => panic!()
                }
            });
            let table = Table::new(
                tree.clone(),
                grid,
                vec![1.0, 2.0],
                vec![1.0, 4.0, 3.0, 2.0],
                Grid::new((2, 5), |_, _| Stroke::Narrow),
                Grid::new((3, 4), |_, _| Stroke::Double),
            );
            Example {
                model,
                buttons,
                checkboxes,
                labels,
                table,
            }
        });
        let mut write1 = result.write();
        let write = &mut *write1;
        write.add(write.table.clone());
        mem::drop(write1);
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
        let mut table = self.table.write();
        table.layout(constraint);
        Layout { size: table.size(), line_settings: Default::default() }
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
    run_local(|tree| Example::new_gui(tree));
}
