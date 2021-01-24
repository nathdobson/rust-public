#![allow(unused_imports, unused_variables)]
#![feature(arbitrary_self_types)]

use std::collections::{HashMap, HashSet};

use termio::gui::gui::{Gui, InputEvent};
use termio::input::{Event, Key, Mouse};
use futures::executor::{ThreadPool, block_on};
use termio::color::Color;
use termio::gui::button::Button;
use termio::canvas::Canvas;
use termio::output::{Foreground, DoubleHeightTop, DoubleHeightBottom};
use termio::input::modifiers::*;
use util::{swrite, Name};
use termio::gui::event;
use std::{process, mem, io};
use netcatd::tcp::{NetcatServer, Model, NetcatServerBuilder};
use termio::gui::layout::{Constraint, Layout};
use termio::screen::Style;
use util::grid::Grid;
use std::sync::{Arc};
use termio::gui::event::{EventSender, SharedGuiEvent};
use util::shutdown::join_to_main;
use std::time::Duration;
use termio::gui::tree::Tree;
use termio::gui::div::{DivRc, Div, DivImpl};
use util::mutrc::MutRc;
use async_util::Mutex;
use async_util::Executor;
use futures::executor::LocalPool;
use async_util::cancel::Cancel;
use futures::task::{Spawn, SpawnExt};

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
    fn add_peer(&mut self, username: &Name, tree: Tree) -> MutRc<Gui> {
        MutRc::new(Gui::new(tree.clone(), Root::new(tree)))
    }

    fn remove_peer(&mut self, username: &Name) {
        println!("Removing {}", username);
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
    let builder = NetcatServerBuilder::new();
    let model =
        MutRc::new(DemoModel::new(builder.event_sender.clone()));
    builder.run_main("0.0.0.0:8000", model);
}