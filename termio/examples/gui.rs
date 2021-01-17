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
use termio::gui::context::{Context, GuiEvent, SharedGuiEvent};
use termio::gui::view::{View, ViewImpl};
use termio::gui::layout::{Constraint, Layout};
use util::lossy;
use std::str;

#[derive(Debug)]
struct Example {
    content: Vec<StyleString>,
    buttons: Vec<View<Button>>,
    labels: Vec<View<Label>>,
}

impl ViewImpl for Example {
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
    let (context, events) = Context::new(Box::new(move || {
        dirty_sender.send(());
    }));
    let root = NodeStrong::<Example>::root(context.clone());
    let mut content = vec!["a".to_style_string()];
    let mut labels = vec![
        Label::new(root.child(
            |v| &v.labels[0],
            |v| &mut v.labels[0])),
        Label::new(root.child(
            |v| &v.labels[1],
            |v| &mut v.labels[1]))
    ];
    labels[0].sync(&content);
    labels[0].set_size((40, 10));
    labels[1].set_size((10, 10));
    let mut buttons =
        ["a", "bb", "ccc", "dddd"].iter().enumerate()
            .map(|(i, s)| {
                let ss = s.to_style_string();
                Button::new(
                    root.child(move |v| &v.buttons[i],
                               move |v| &mut v.buttons[i]),
                    s.to_string(),
                    root.new_shared_event(
                        move |e| {
                            e.content.push(ss.clone());
                            let e = &mut **e;
                            e.labels[0].sync(&e.content);
                        }
                    ),
                )
            }).collect();
    let mut gui =
        Gui::new(Box::new(View::new(root, Example {
            content,
            buttons,
            labels,
        })));
    gui.set_background(Style {
        background: Color::Gray24(23),
        foreground: Color::Gray24(0),
        ..Style::default()
    });
    let gui = Arc::new(Mutex::new(gui));
    events.start(gui.clone());

    thread::spawn({
        let context = context.clone();
        move || {
            let mut reader = EventReader::new(stdin());
            loop {
                let next = reader.read().unwrap();
                if next == Event::KeyEvent(KeyEvent::typed('c').control()) {
                    eprintln!("Quitting");
                    process::exit(0);
                }
                context.run(GuiEvent::new(move |gui| {
                    gui.handle(&next)
                }))
            }
        }
    });

    thread::spawn(move || {
        let stdout = stdout();
        let mut buffer = vec![];
        for () in dirty_receiver {
            {
                let mut gui = gui.lock().unwrap();
                gui.paint_buffer(&mut buffer);
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