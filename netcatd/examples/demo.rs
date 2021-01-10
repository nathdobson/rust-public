#![allow(unused_imports, unused_variables)]

use std::collections::{HashMap, HashSet};

use termio::gui::gui::{Gui, InputEvent, OutputEvent, OutputEventTrait};
use termio::input::{Event, Key, Mouse};

use termio::color::Color;
use termio::gui::button::Button;
use termio::canvas::Canvas;
use termio::output::{Foreground, DoubleHeightTop, DoubleHeightBottom};
use termio::input::modifiers::*;
use util::{swrite, Name};

use std::{process, mem};
use netcatd::tcp::NetcatServer;
use termio::gui::node::{Node, NodeImpl};
use termio::gui::layout::Constraint;
use termio::screen::Style;
use termio::gui::table::{TableImpl, Table};
use util::grid::Grid;
use netcatd::{Handler, EventLoop};
use std::sync::{Arc, Mutex};
use termio::gui::group::Group;

pub struct DemoHandler {
    server: Arc<dyn EventLoop>,
    gui: HashMap<Name, Gui<Group<Table<Root>>>>,
}

impl DemoHandler {
    pub fn new(server: Arc<NetcatServer>) -> Self {
        DemoHandler { server, gui: HashMap::new() }
    }
}

#[derive(Debug)]
struct Root {
    hello: Node<Button>,
    goodbye: Node<Button>,
}

#[derive(Debug)]
struct Hello;

impl OutputEventTrait for Hello {}

#[derive(Debug)]
struct Goodbye;

impl OutputEventTrait for Goodbye {}

impl Root {
    fn new() -> Self {
        Root {
            hello: Button::new("hello".to_string(), Arc::new(Hello)),
            goodbye: Button::new("goodbye".to_string(), Arc::new(Goodbye)),
        }
    }
}

impl TableImpl for Root {
    fn table_children(this: &Node<Group<Table<Self>>>) -> Grid<&Node> {
        Grid::from_iterator(
            (2, 1),
            vec![&this.hello as &Node, &this.goodbye].into_iter())
    }

    fn table_children_mut(this: &mut Node<Group<Table<Self>>>) -> Grid<&mut Node> {
        let this = &mut ****this;
        Grid::from_iterator(
            (2, 1),
            vec![&mut this.hello as &mut Node, &mut this.goodbye].into_iter())
    }
}

impl Handler for DemoHandler {
    fn peer_add(&mut self, username: &Name) {
        let root = Root::new();
        let mut gui = Gui::new(Table::new(root));
        gui.update_text_size();
        gui.set_background(Style { background: Color::Gray24(23), foreground: Color::Gray24(0), ..Style::default() });
        self.gui.insert(username.clone(), gui);
        self.server.peer_render(username);
    }

    fn peer_shutdown(&mut self, username: &Name) {
        self.gui.get_mut(username).unwrap().set_enabled(false);
        self.server.peer_render(username);
    }

    fn peer_close(&mut self, username: &Name) {
        self.gui.remove(username);
    }

    fn peer_event(&mut self, username: &Name, event: &Event) {
        println!("{:?}", event);
        if let Event::KeyEvent(event) = event {
            if let Key::Type('c') = event.key {
                if event.modifier == CONTROL {
                    self.server.peer_shutdown(username);
                }
            }
        }
        let gui = self.gui.get_mut(username).unwrap();
        let mut outputs = vec![];
        gui.handle(event, &mut outputs);
        println!("{:?}", outputs);
        self.server.peer_render(username);
    }

    fn peer_render(&mut self, username: &Name, output: &mut Vec<u8>) {
        let gui = self.gui.get_mut(username).unwrap();
        gui.paint();
        let buffer = gui.buffer();
        println!("{:?}", String::from_utf8_lossy(buffer.as_slice()));
        assert_eq!(output.len(), 0);
        mem::swap(buffer, output);
    }
}

fn main() {
    println!("Binding 0.0.0.0:8000");
    let server = NetcatServer::new("0.0.0.0:8000").unwrap();
    let (ctx, _canc, rec) = util::cancel::channel();
    let handler = Arc::new(Mutex::new(DemoHandler::new(server.clone())));
    server.listen(ctx, handler).unwrap();
    rec.recv().unwrap();
}