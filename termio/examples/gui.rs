#![feature(never_type)]
#![allow(unused_imports)]

use termio::gui::gui::{Gui, OutputEventTrait};
use termio::gui::table::{Table, TableImpl};
use termio::gui::button::Button;
use std::sync::{Arc, mpsc};
use termio::gui::node::Node;
use termio::input::{EventReader, Event, KeyEvent};
use std::io::{stdin, stdout, Write};
use std::error::Error;
use std::{mem, thread, process};
use util::grid::Grid;
use termio::gui::label::Label;
use termio::string::{StyleFormatExt};
use util::any::{Upcast, AnyExt};
use std::any::Any;
use std::ops::Deref;
use termio::screen::Style;
use termio::color::Color;
use termio::gui::time::TimeEvent;
use timer::Timer;
use std::time::Instant;
use chrono;
use std::time;
use termio::gui::group::Group;

#[derive(Debug)]
struct Example {
    buttons: [Node<Button>; 4],
    labels: [Node<Label>; 2],
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Click(&'static str);

impl OutputEventTrait for Click {}

impl TableImpl for Example {
    fn table_children(this: &Node<Group<Table<Self>>>) -> Grid<&Node> {
        Grid::from_iterator(
            (2, 3),
            this.buttons.iter().map(|x| x as &Node)
                .chain(this.labels.iter().map(|x| x as &Node)))
    }

    fn table_children_mut(this: &mut Node<Group<Table<Self>>>) -> Grid<&mut Node> {
        let this = &mut ****this;
        Grid::from_iterator(
            (2, 3),
            this.buttons.iter_mut().map(|x| x as &mut Node)
                .chain(this.labels.iter_mut().map(|x| x as &mut Node)))
    }
}

fn main() {
    main_impl().unwrap();
}

fn main_impl() -> Result<(), Box<dyn Error>> {
    let mut content = vec!["a".to_style_string()];
    let mut label1 = Label::new();
    label1.sync(&content);
    label1.set_size((40, 10));
    let mut label2 = Label::new();
    label2.set_size((10, 10));
    let mut gui =
        Gui::new(Table::new(Example {
            buttons: [
                Button::new("aaa".to_string(), Arc::new(Click("a"))),
                Button::new("bbbbbbb".to_string(), Arc::new(Click("b"))),
                Button::new("cccccccccc".to_string(), Arc::new(Click("c"))),
                Button::new("ddddddddddddddddd".to_string(), Arc::new(Click("d"))),
            ],
            labels: [
                label1,
                label2,
            ],
        }));
    gui.set_background(Style {
        background: Color::Gray24(23),
        foreground: Color::Gray24(0),
        ..Style::default()
    });
    let (events, event_receiver) = mpsc::channel();

    thread::spawn({
        let events = events.clone();
        move || {
            let mut reader = EventReader::new(stdin());
            loop {
                let next = reader.read().unwrap();
                if next == Event::KeyEvent(KeyEvent::typed('c').control()) {
                    eprintln!("Quitting");
                    process::exit(0);
                }
                events.send(next).unwrap();
            }
        }
    });

    thread::spawn({
        let events = events.clone();
        move || {
            let timer = Timer::new();
            let stdout = stdout();
            loop {
                let output = gui.buffer();
                let mut lock = stdout.lock();
                lock.write_all(&output).unwrap();
                lock.flush().unwrap();
                output.clear();
                mem::drop(lock);

                let next = if let Ok(next) = event_receiver.recv() {
                    next
                } else {
                    break;
                };
                let mut output_events = vec![];
                gui.handle(&next, &mut output_events);
                for o in output_events {
                    if let Ok(c) = o.downcast_event::<Click>() {
                        let v = gui.buttons[3].visible();
                        gui.buttons[3].set_visible(!v);
                        content.push(c.0.to_style_string());
                        gui.labels[0].sync(&content);
                    } else if let Ok(TimeEvent(when)) = o.downcast_event::<TimeEvent>() {
                        let when = *when;
                        let delay = chrono::Duration::from_std(when - Instant::now()).unwrap();
                        timer.schedule_with_delay(
                            delay,
                            {
                                let events = events.clone();
                                move || {
                                    events.send(Event::Time(when)).ok();
                                }
                            }).ignore();
                    } else {
                        eprintln!("Unknown {:?}", o);
                    }
                }
                gui.paint();
            }
        }
    }).join().unwrap();
    println!("Done");
    Ok(())
}