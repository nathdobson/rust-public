#![feature(never_type, arbitrary_self_types)]
#![allow(unused_imports)]

use termio::gui::gui::{Gui};
use termio::gui::button::Button;
use std::sync::{Arc, mpsc, Mutex};
use termio::gui::node::{Node, NodeStrong};
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
use termio::gui::view::{View, ViewImpl};
use termio::gui::layout::{Constraint, Layout};
use util::lossy;
use std::str;
use termio::gui::event::{Priority, GuiEvent, SharedGuiEvent};
use termio::gui::event;
use termio::gui::controller::Controller;

struct ExampleController {
    model: Vec<StyleString>,
    gui: Gui,
}

#[derive(Debug)]
struct ExampleView {
    buttons: Vec<View<Button>>,
    labels: Vec<View<Label>>,
}

impl Controller for ExampleController {}

impl ViewImpl for ExampleView {
    fn layout_impl(self: &mut View<Self>, constraint: &Constraint) -> Layout {
        let grid = Grid::from_iterator(
            (2, 3),
            self
                .buttons.iter().map(|x| x.node_strong().downgrade())
                .chain(self.labels.iter().map(|x| x.node_strong().downgrade())
                ),
        );
        constraint.table_layout(
            self,
            &grid,
        )
    }
}

fn main() {
    main_impl().unwrap();
}

fn main_impl() -> Result<(), Box<dyn Error>> {
    let (dirty_sender, dirty_receiver) = lossy::channel();
    let (event_sender, event_receiver) = event::channel();
    let root = NodeStrong::<ExampleView>::root(
        event_sender.clone(),
        move || { dirty_sender.send(()); },
        |controller: &ExampleController| { &controller.gui },
        |controller: &mut ExampleController| { &mut controller.gui });
    let model = vec!["a".to_style_string()];
    let mut labels = vec![
        Label::new(root.child(
            |v| &v.labels[0],
            |v| &mut v.labels[0])),
        Label::new(root.child(
            |v| &v.labels[1],
            |v| &mut v.labels[1]))
    ];
    labels[0].sync(&model);
    labels[0].set_size((40, 10));
    labels[1].set_size((10, 10));
    let buttons =
        ["a", "bb", "ccc", "dddd"].iter().enumerate()
            .map(|(i, s)| {
                let ss = s.to_style_string();
                Button::new(
                    root.child(move |v| &v.buttons[i],
                               move |v| &mut v.buttons[i]),
                    s.to_string(),
                    SharedGuiEvent::new(move |c: &mut ExampleController| {
                        c.model.push(ss.clone());
                        c.gui.root_mut().downcast_view_mut::<ExampleView>().labels[0].sync(&c.model);
                    }),
                )
            }).collect();
    let mut gui =
        Gui::new(Box::new(View::new(root, ExampleView {
            buttons,
            labels,
        })));
    gui.set_background(Style {
        background: Color::Gray24(23),
        foreground: Color::Gray24(0),
        ..Style::default()
    });
    let controller = ExampleController { model, gui };
    let controller = Arc::new(Mutex::new(controller));
    event_receiver.start(controller.clone());

    thread::spawn({
        let event_sender = event_sender.clone();
        move || {
            let mut reader = EventReader::new(stdin());
            loop {
                let next = reader.read().unwrap();
                if next == Event::KeyEvent(KeyEvent::typed('c').control()) {
                    eprintln!("Quitting");
                    process::exit(0);
                }
                event_sender.run(Priority::Later, GuiEvent::new(move |controller: &mut ExampleController| {
                    controller.gui.handle(&next)
                }))
            }
        }
    });

    thread::spawn(move || {
        let stdout = stdout();
        let mut buffer = vec![];
        for () in dirty_receiver {
            {
                let mut controller = controller.lock().unwrap();
                controller.gui.paint_buffer(&mut buffer);
            }
            {
                let mut lock = stdout.lock();
                eprintln!("{:?}", str::from_utf8(&buffer));
                lock.write_all(&buffer).unwrap();
                lock.flush().unwrap();
            }
        }
    }).join().unwrap();


    println!("Done");
    Ok(())
}