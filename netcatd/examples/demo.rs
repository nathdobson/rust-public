#![allow(unused_imports, unused_variables)]
#![feature(arbitrary_self_types)]

use std::collections::{HashMap, HashSet};

use termio::gui::gui::{Gui, InputEvent};
use termio::input::{Event, Key, Mouse};

use termio::color::Color;
use termio::gui::button::Button;
use termio::canvas::Canvas;
use termio::output::{Foreground, DoubleHeightTop, DoubleHeightBottom};
use termio::input::modifiers::*;
use util::{swrite, Name};

use std::{process, mem};
use netcatd::tcp::{NetcatServer, Model, ModelExt};
use termio::gui::node::{Node, NodeStrong};
use termio::gui::layout::{Constraint, Layout};
use termio::screen::Style;
use util::grid::Grid;
use std::sync::{Arc, Mutex};
use termio::gui::event::{EventSender, SharedGuiEvent};
use termio::gui::view::{View, ViewImpl};

#[derive(Debug)]
pub struct DemoModel {
    sender: EventSender,
}

impl DemoModel {
    pub fn new(sender: EventSender) -> Self {
        DemoModel { sender }
    }
}

#[derive(Debug)]
struct Root {
    hello: View<Button>,
    goodbye: View<Button>,
}

impl Root {
    fn new(id: NodeStrong<Root>) -> View<Self> {
        View::new(id.clone(), Root {
            hello: Button::new(
                id.child(
                    |r| &r.hello,
                    |r| &mut r.hello),
                "hello".to_string(),
                DemoModel::new_shared_event(|m|
                    println!("{:?} says hello", m))),
            goodbye: Button::new(
                id.child(
                    |r| &r.goodbye,
                    |r| &mut r.goodbye),
                "goodbye".to_string(),
                DemoModel::new_shared_event(|m|
                    println!("{:?} says goodbye", m))),
        })
    }
}

impl Model for DemoModel {
    fn make_gui(&mut self, username: &Name, node: NodeStrong) -> Gui {
        Gui::new(Box::new(Root::new(node.downcast_node())))
    }
}

impl ViewImpl for Root {
    fn layout_impl(self: &mut View<Self>, constraint: &Constraint) -> Layout {
        let grid = Grid::from_iterator((1, 2), vec![self.hello.node().upcast_node(), ].into_iter());
        constraint.table_layout(self, &grid)
    }
}

fn main() {
    println!("Binding 0.0.0.0:8000");
    let server =
        NetcatServer::new(
            "0.0.0.0:8000",
            |event_sender| {
                DemoModel::new(event_sender)
            }).unwrap();
    let (ctx, _canc, rec) = util::cancel::channel();
    server.listen(ctx).unwrap();
    rec.recv().unwrap();
}