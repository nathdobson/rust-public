#![feature(never_type)]

use termio::gui::gui::{Gui, OutputEventTrait};
use termio::gui::table::{Table, TableImpl};
use termio::gui::container::Container;
use termio::gui::button::Button;
use std::sync::Arc;
use termio::gui::node::Node;
use termio::input::{EventReader, Event, KeyEvent};
use std::io::{stdin, stdout, Write};
use std::error::Error;
use std::mem;
use util::grid::Grid;

struct Example {
    buttons: [Button; 4],
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Click;

impl OutputEventTrait for Click {}

impl TableImpl for Example {
    fn children(&self) -> Grid<&dyn Node> {
        Grid::from_iterator((2, 2), self.buttons.iter().map(|x| x as &dyn Node))
    }

    fn children_mut(&mut self) -> Grid<&mut dyn Node> {
        Grid::from_iterator((2, 2), self.buttons.iter_mut().map(|x| x as &mut dyn Node))
    }
}

fn main() {
    main_impl().unwrap();
}

fn main_impl() -> Result<!, Box<dyn Error>> {
    let mut gui =
        Gui::new(Container::new(Table::new(Example {
            buttons: [
                Button::new("aaa".to_string(), Arc::new(Click)),
                Button::new("bbbbbbb".to_string(), Arc::new(Click)),
                Button::new("cccccccccc".to_string(), Arc::new(Click)),
                Button::new("ddddddddddddddddd".to_string(), Arc::new(Click)),
            ],
        })));
    gui.update_text_size();
    let mut reader = EventReader::new(stdin());
    let stdout = stdout();
    loop {
        let output = gui.buffer();
        let mut lock = stdout.lock();
        lock.write_all(&output)?;
        lock.flush()?;
        output.clear();
        mem::drop(lock);

        let next = reader.read()?;
        eprintln!("Event {:?}", next);
        if next == Event::KeyEvent(KeyEvent::typed('c').control()) {
            break;
        }
        let mut output_events = vec![];
        gui.handle(&next, &mut output_events);
        eprintln!("{:?}", output_events);
        gui.paint();
    }
    std::process::exit(0);
}