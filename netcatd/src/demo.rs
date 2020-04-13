use std::collections::HashMap;

use termio::gui::{Gui, Node};
use termio::gui::Button;
use termio::input::Event;
use util::object::Object;

use crate::{NetcatHandler, NetcatPeer};

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
        gui.background = Some(16);
        let mut button = Box::new(Button::new(format!("Hello!")));
        button.header_mut().position = (10, 10);
        gui.add_node(button);
        gui.paint();
        self.gui.insert(id.clone(), gui);
//        thread::spawn(move || {
//            for i in 0.. {
//                let theta = (i as f32) / 100.0;
//                let width = (((theta.cos().abs()) * 1440.0) as usize).max(200);
//                let height = ((theta.sin().abs()) * 800.0) as usize;
//                let x = (2880 / 2 - width) / 2;
//                let y = (1800 / 2 - height) / 2;
//                thread::sleep(Duration::from_millis(10));
//                write!(peer, "{}{}", MoveWindow(x, y), ResizeWindow(width, height));
//                peer.flush();
//            }
//        });
    }

    fn remove_peer(&mut self, _: &Object) {}

    fn handle_event(&mut self, id: &Object, event: &Event) {
        if let Some(event) = self.gui.get_mut(id).unwrap().handle_event(event) {
            println!("{:?}", event);
        }
    }
}
