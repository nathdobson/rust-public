#![feature(never_type, arbitrary_self_types)]
#![feature(box_syntax)]
#![allow(unused_imports)]
#![deny(unused_must_use)]

use termio::gui::gui::{Gui, InputEvent};
use termio::gui::button::Button;
use std::sync::{Arc, mpsc, Mutex};
use termio::input::{EventReader, Event, KeyEvent};
use std::io::{Write};
use tokio::io::stdin;
use tokio::io::stdout;
use std::error::Error;
use std::{mem, thread, process, io};
use util::grid::Grid;
use termio::gui::label::Label;
use termio::string::{StyleFormatExt, StyleString};
use std::any::Any;
use std::ops::Deref;
use termio::screen::{Style, Rune};
use termio::color::Color;
use std::time::Instant;
use std::time;
use termio::gui::tree::{Tree, TreeReceiver};
use termio::gui::layout::{Constraint, Layout};
use util::lossy;
use std::str;
use termio::gui::event::{GuiEvent, BoxFnMut};
use termio::gui::{event, GuiBuilder};
use termio::gui::div::{Div, DivImpl, DivRc, DivWeak};
use util::atomic_refcell::AtomicRefCell;
use util::mutrc::MutRc;
use termio::gui::checkbox::CheckBox;
use termio::gui::field::Field;
use termio::gui::table::{Table, TableDiv};
use termio::line::Stroke;
use termio::canvas::Canvas;
use async_backtrace::traced_main;
use tokio::sync::mpsc::{channel, unbounded_channel};
use tokio_stream::wrappers::UnboundedReceiverStream;
use async_util::poll::poll_loop;

#[derive(Debug)]
struct Example {
    model: Vec<StyleString>,
    buttons: Vec<DivRc<Button>>,
    checkbox: DivRc<CheckBox>,
    field: DivRc<Field>,
    labels: Vec<DivRc<Label>>,
    table: DivRc<Table>,
    button_rx: Option<UnboundedReceiverStream<StyleString>>,
}

impl Example {
    fn new(tree: Tree) -> DivRc<Self> {
        let (button_tx, button_rx) = unbounded_channel();

        let model = vec!["a".to_style_string()];
        let mut labels = vec![Label::new(tree.clone()), Label::new(tree.clone())];
        labels[0].write().sync_log(&model);
        labels[0].write().set_size((40, 10));
        labels[1].write().set_size((10, 10));
        let buttons: Vec<DivRc<Button>> =
            ["a", "bb", "ccc", "dddd"].iter()
                .map(|s| {
                    let ss = s.to_style_string();
                    let button_tx = button_tx.clone();
                    Button::new(
                        tree.clone(),
                        s.to_string(),
                        BoxFnMut::new(move || { button_tx.send(ss.clone()).ok(); }),
                    )
                }).collect();
        let checkbox: DivRc<CheckBox> =
            CheckBox::new(
                tree.clone(),
                "x".to_style_string(),
                false,
                BoxFnMut::new(move || { button_tx.send("x".to_style_string()).ok(); }),
            );
        let field: DivRc<Field> = Field::new(tree.clone(), "text".to_string());
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
                    div: checkbox.clone(),
                    flex: false,
                    align: (0.0, 0.0),
                },
                (1, 2) => TableDiv {
                    div: field.clone(),
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
        let mut result = DivRc::new(tree.clone(), Example {
            model,
            buttons,
            checkbox,
            field,
            labels,
            table,
            button_rx: Some(UnboundedReceiverStream::new(button_rx)),
        });
        let mut write1 = result.write();
        let write = &mut *write1;
        write.add(write.table.clone());
        mem::drop(write1);
        result
    }
    fn new_gui() -> MutRc<Gui> {
        let mut builder = GuiBuilder::new();
        let div = Example::new(builder.tree().clone());
        let mut gui = builder.build(div);
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
        } else if let InputEvent::KeyEvent(_) = event {
            self.field.write().handle(event);
            return true;
        }
        false
    }
}

fn main() {
    traced_main("127.0.0.1:9998".to_string(), async move {
        let mut gui = Example::new_gui();
        match poll_loop(|cx| {
            gui.write().poll_elapse(cx)
        }).await {
            Err(e) => {
                if e.kind() != io::ErrorKind::Interrupted {
                    eprintln!("IO error: {}", e);
                }
            }
            Ok(x) => match x {}
        };
    });
}
