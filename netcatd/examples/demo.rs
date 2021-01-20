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
use termio::gui::event;
use std::{process, mem};
use netcatd::tcp::{NetcatServer, Model};
use termio::gui::layout::{Constraint, Layout};
use termio::screen::Style;
use util::grid::Grid;
use std::sync::{Arc, Mutex};
use termio::gui::event::{EventSender, SharedGuiEvent};
use util::shutdown::join_to_main;
use std::time::Duration;
use termio::gui::tree::Tree;
use termio::gui::div::{DivRc, Div, DivImpl};
use util::mutrc::MutRc;

#[derive(Debug)]
pub struct DemoModel {}

impl DemoModel {
    pub fn new(sender: EventSender) -> Self {
        DemoModel {}
    }
}

#[derive(Debug)]
struct Root {
    hello: DivRc<Button>,
    goodbye: DivRc<Button>,
}

impl Root {
    fn new(tree: Tree) -> DivRc<Self> {
        let mut result = DivRc::new(tree.clone(), Root {
            hello: Button::new(
                tree.clone(),
                "hello".to_string(),
                SharedGuiEvent::new(||
                    println!("Hello"))),
            goodbye: Button::new(
                tree.clone(),
                "goodbye".to_string(),
                SharedGuiEvent::new(||
                    println!("Goodbye"))),
        });
        let mut write = result.write();
        let hello = write.hello.clone();
        let goodbye = write.goodbye.clone();
        write.add(hello);
        write.add(goodbye);
        mem::drop(write);
        result
    }
}

impl Model for DemoModel {
    fn make_gui(&mut self, username: &Name, tree: Tree) -> MutRc<Gui> {
        MutRc::new(Gui::new(tree.clone(), Root::new(tree)))
    }
}

impl DivImpl for Root {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        let mut grid = Grid::from_iterator(
            (1, 2),
            vec![
                self.hello.clone().upcast_div(),
                self.goodbye.clone().upcast_div(),
            ].into_iter());
        constraint.table_layout(&mut grid)
    }
}

fn main() {
    println!("Binding 0.0.0.0:8000");
    let (event_mutex, event_sender, event_joiner) = event::event_loop();
    let server =
        NetcatServer::new(
            "0.0.0.0:8000",
            event_mutex,
            event_sender.clone(),
            MutRc::new(DemoModel::new(event_sender)),
        ).unwrap();
    let (ctx, canceller, receiver) = util::cancel::channel();
    ctx.spawn({
        let ctx = ctx.clone();
        move || {
            server.listen(ctx).unwrap();
            Ok(())
        }
    });
    mem::drop(ctx);
    join_to_main(canceller, receiver, Duration::from_secs(60));
}