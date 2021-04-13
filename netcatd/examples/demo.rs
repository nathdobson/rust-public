#![allow(unused_imports, unused_variables)]
#![feature(arbitrary_self_types)]

use std::{io, mem, process};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_util::coop::Cancel;
use netcatd::tcp::{Model, NetcatServer, NetcatServerBuilder};
use termio::canvas::Canvas;
use termio::color::Color;
use termio::gui::button::Button;
use termio::gui::div::{Div, DivImpl, DivRc};
use termio::gui::event;
use termio::gui::event::{EventSender, SharedGuiEvent};
use termio::gui::gui::{Gui, InputEvent};
use termio::gui::layout::{Constraint, Layout};
use termio::gui::tree::Tree;
use termio::input::{Event, Key, Mouse, KeyEvent};
use termio::input::modifiers::*;
use termio::output::{DoubleHeightBottom, DoubleHeightTop, Foreground};
use termio::screen::Style;
use util::{Name, swrite};
use util::grid::Grid;
use util::mutrc::MutRc;
use termio::gui::table::{Table, TableDiv};
use termio::line::Stroke;
use async_backtrace::traced_main;

#[derive(Debug)]
pub struct DemoModel {}

impl DemoModel {
    pub fn new() -> Self {
        DemoModel {}
    }
}

#[derive(Debug)]
struct Root {
    hello: DivRc<Button>,
    goodbye: DivRc<Button>,
    table: DivRc<Table>,
}

impl Root {
    fn new(tree: Tree) -> DivRc<Self> {
        let hello = Button::new(
            tree.clone(),
            "hello".to_string(),
            SharedGuiEvent::new(||
                println!("Hello")));
        let goodbye = Button::new(
            tree.clone(),
            "goodbye".to_string(),
            SharedGuiEvent::new(||
                println!("Goodbye")));
        let grid = Grid::new((2, 1), |x, y| {
            match (x, y) {
                (0, 0) => TableDiv {
                    div: hello.clone(),
                    flex: true,
                    align: (0.0, 0.0),
                },
                (1, 0) => TableDiv {
                    div: goodbye.clone(),
                    flex: false,
                    align: (0.0, 0.0),
                },
                _ => panic!()
            }
        });
        let table = Table::new(
            tree.clone(),
            grid,
            vec![1.0, 2.0],
            vec![1.0],
            Grid::new((2, 2), |_, _| Stroke::Narrow),
            Grid::new((3, 1), |_, _| Stroke::Double),
        );
        let mut result = DivRc::new(tree.clone(), Root {
            hello,
            goodbye,
            table: table.clone(),
        });
        result.write().add(table);
        result
    }
}

impl Model for DemoModel {
    fn add_peer(&mut self, username: &Name, tree: Tree) -> MutRc<Gui> {
        MutRc::new(Gui::new(tree.clone(), Root::new(tree)))
    }

    fn remove_peer(&mut self, username: &Name) {}
}

impl DivImpl for Root {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        let mut table = self.table.write();
        table.layout(constraint);
        Layout { size: table.size(), line_settings: Default::default() }
    }
}

fn main() {
    traced_main("127.0.0.1:9999".to_string(), async move {
        let (builder, runner) = NetcatServerBuilder::new();
        let model =
            MutRc::new(DemoModel::new());
        builder.build_main("0.0.0.0:8000", model);
        runner.await;
    });
}