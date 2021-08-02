#![allow(unused_imports, unused_variables)]
#![feature(arbitrary_self_types)]
#![deny(unused_must_use)]

use std::{io, mem, process};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_util::coop::Cancel;
use netcatd::tcp::{NetcatServer, Model};
use termio::canvas::Canvas;
use termio::color::Color;
use termio::gui::button::Button;
use termio::gui::div::{Div, DivImpl, DivRc};
use termio::gui::{event, GuiBuilder};
use termio::gui::event::{BoxFnMut};
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
use async_util::poll::poll_loop;

#[derive(Debug)]
pub struct DemoModel {}

impl DemoModel {
    pub fn new() -> Self {
        DemoModel {}
    }
}

impl Model for DemoModel {
    fn make_peer(&mut self, name: &Name, mut builder: GuiBuilder) -> Gui {
        let div = Root::new(builder.tree());
        builder.build(div)
    }

    fn remove_peer(&mut self, name: &Name) {
        println!("Demo remove peer {}", name);
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
            BoxFnMut::new(||
                println!("Hello")));
        let goodbye = Button::new(
            tree.clone(),
            "goodbye".to_string(),
            BoxFnMut::new(||
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

impl DivImpl for Root {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        let mut table = self.table.write();
        table.layout(constraint);
        Layout { size: table.size(), line_settings: Default::default() }
    }
}

fn main() {
    traced_main("127.0.0.1:9999".to_string(), async move {
        let cancel = Cancel::new();
        cancel.clone().run_main(async {
            let (listener, mut server) = NetcatServer::new(cancel);
            listener.listen("0.0.0.0:8000").await?;
            let mut model = DemoModel::new();
            poll_loop(|cx| {
                server.poll_elapse(cx, &mut model)
            }).await?;
            #[allow(unreachable_code)]
                Ok::<(), io::Error>(())
        }).await;
    });
}