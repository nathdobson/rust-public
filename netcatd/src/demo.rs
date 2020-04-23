use std::collections::{HashMap, HashSet};

use termio::gui::{Gui, NodeHeader, IsNode, NodeEvent};
use termio::input::{Event, Key, Mouse};

use crate::{NetcatHandler, NetcatPeer};
use termio::color::Color;
use termio::gui::button::Button;
use termio::gui::Node;
use util::shared::{HasHeaderExt, Header, HasHeader};
use termio::canvas::Canvas;
use termio::output::{Foreground, DoubleHeightTop, DoubleHeightBottom};
use termio::input::modifiers::*;
use std::process;

pub struct DemoHandler {
    gui: HashMap<NetcatPeer, Gui>,
}

impl DemoHandler {
    pub fn new() -> Self {
        DemoHandler { gui: HashMap::new() }
    }
}

#[derive(Debug)]
struct Hand {
    header: Header<Self, NodeHeader>,
    length: usize,
    hover: Option<usize>,
    selected: HashSet<usize>,
}

impl HasHeader<NodeHeader> for Hand {
    fn shared_header(&self) -> &Header<Self, NodeHeader> { &self.header }

    fn shared_header_mut(&mut self) -> &mut Header<Self, NodeHeader> { &mut self.header }
}

impl IsNode for Hand {
    fn paint(&self, w: &mut Canvas) {
        swrite!(w.writer, "{}", Foreground(Color::Gray24(0)));
        let xp = 0;
        w.draw((xp, 0), DoubleHeightTop);
        w.draw((xp, 1), DoubleHeightBottom);
        w.draw((xp, 2), DoubleHeightTop);
        w.draw((xp, 3), DoubleHeightTop);
        w.draw((xp, 4), DoubleHeightBottom);
        w.draw((xp, 5), DoubleHeightTop);
        w.draw((xp, 6), DoubleHeightTop);
        w.draw((xp, 7), DoubleHeightBottom);
        w.draw((xp, 8), DoubleHeightTop);
        w.draw((xp, 9), DoubleHeightTop);
        for y in 0..10 {
            w.draw((xp, y), &format!("{}", " ".repeat(self.size().0 as usize)));
        }
        for i in 0..self.length {
            let xp = (i * 3) as isize;
            let yp = if self.selected.contains(&i) { 0 } else { 3 };
            w.draw((xp, 0 + yp), &format!("{}", "╭──╮"));
            w.draw((xp, 1 + yp), &format!("{}", "╭──╮"));
            w.draw((xp, 2 + yp), &format!("{}", "│  │"));
            w.draw((xp, 3 + yp), &format!("{}", "│ab│"));
            w.draw((xp, 4 + yp), &format!("{}", "│ab│"));
            w.draw((xp, 5 + yp), &format!("{}", "│  │"));
            w.draw((xp, 6 + yp), &format!("{}", "╰──╯"));
        }
    }

    fn handle_event(&mut self, event: &Event) -> Option<NodeEvent> {
        match event {
            Event::MouseEvent(mouse_event) => {
                if self.bounds().contains(mouse_event.position) {
                    let x = ((mouse_event.position.0 - self.position().0) / 3) as usize;
                    if !mouse_event.motion && mouse_event.mouse == Mouse::Down(0) {
                        if !self.selected.insert(x) {
                            self.selected.remove(&x);
                        }
                    }
                } else {
                    self.hover = None;
                }
            }
            _ => {}
        }
        None
    }

    fn size(&self) -> (isize, isize) {
        (((3 * self.length) + 1) as isize, 10)
    }
}

impl Hand {
    fn new(length: usize) -> Node<Hand> {
        Header::new_shared(Hand {
            header: Header::new_header(NodeHeader::new()),
            length,
            hover: None,
            selected: HashSet::new(),
        })
    }
}

impl NetcatHandler for DemoHandler {
    fn add_peer(&mut self, peer: &NetcatPeer) {
        let mut gui = Gui::new(Box::new(peer.clone()));
        gui.background = Some(Color::RGB666(0, 0, 0));
        let button1 = Button::new(format!("Hello!"));
        button1.borrow_mut().header_mut().position = (10, 13);
        gui.add_node(button1);
        let button2 = Button::new(format!("Goodbye!"));
        button2.borrow_mut().header_mut().position = (10, 16);
        gui.add_node(button2);
        let hand = Hand::new(10);
        hand.borrow_mut().header_mut().position = (3, 3);
        gui.add_node(hand);
        gui.paint();
        self.gui.insert(peer.clone(), gui);
    }

    fn remove_peer(&mut self, _: &NetcatPeer) {}

    fn handle_event(&mut self, peer: &NetcatPeer, event: &Event) {
        println!("{:?}", event);
        if let Event::KeyEvent(event) = event {
            if let Key::Type('c') = event.key {
                if event.modifier == CONTROL {
                    process::exit(0);
                }
            }
        }
        let gui = self.gui.get_mut(peer).unwrap();
        if let Some(event) = gui.handle_event(event) {
            println!("{:?}", event);
        }
        gui.paint();
    }
}
