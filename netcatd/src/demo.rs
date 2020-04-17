use std::collections::HashMap;

use termio::gui::{Gui, Node};
use termio::input::Event;
use util::object::Object;

use crate::{NetcatHandler, NetcatPeer};
use termio::color::Color;
use termio::gui::button::Button;
use std::thread;
use std::time::Duration;

pub struct DemoHandler {
    gui: HashMap<Object, Gui>,
}

impl DemoHandler {
    pub fn new() -> Self {
        DemoHandler { gui: HashMap::new() }
    }
}

impl NetcatHandler for DemoHandler {
    fn add_peer(&mut self, id: &Object, peer: NetcatPeer) {
        let mut gui = Gui::new(Box::new(peer));
        gui.background = Some(Color::RGB666(0, 0, 0));
        let mut button1 = Box::new(Button::new(format!("Hello!")));
        button1.header_mut().position = (10, 10);
        gui.add_node(button1);
        let mut button2 = Box::new(Button::new(format!("Goodbye!")));
        button2.header_mut().position = (20, 20);
        gui.add_node(button2);
        gui.paint();
        self.gui.insert(id.clone(), gui);
    }

    fn remove_peer(&mut self, _: &Object) {}

    fn handle_event(&mut self, id: &Object, event: &Event) {
        let gui = self.gui.get_mut(id).unwrap();
        if let Some(event) = gui.handle_event(event) {
            println!("{:?}", event);
        }
        gui.paint();
    }
}
