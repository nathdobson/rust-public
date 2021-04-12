use std::{io, mem};
use std::future::Future;
use std::iter::FromIterator;
use std::sync::Arc;
use std::time::Instant;

use async_util::coop::Cancel;
use async_util::priority::PriorityRunner;
use util::mutrc::MutRc;

use crate::gui::event::event_loop;
use crate::gui::event::read_loop;
use crate::gui::gui::Gui;
use crate::gui::tree::Tree;
use tokio::pin;
use tokio::io::stdin;
use tokio::io::stdout;
use async_util::futureext::FutureExt;

pub mod layout;
pub mod gui;
pub mod tree;
pub mod button;
pub mod label;
pub mod event;
pub mod div;
pub mod checkbox;
pub mod flow;
pub mod pad;
pub mod table;
pub mod select;
pub mod field;

pub async fn run_local(gui: impl Send + FnOnce(Tree) -> MutRc<Gui>) {
    let cancel = Cancel::new();
    let (event_sender, el) = event_loop();
    let (tree, paint_receiver, layout_receiver) = Tree::new(cancel, event_sender.clone());
    let gui = gui(tree.clone());
    let rl = {
        let event_sender = event_sender.clone();
        let gui = gui.clone();
        async move {
            if let Err(e) = read_loop(event_sender, gui, stdin()).await {
                eprintln!("Read error {:?}", e);
            }
        }
    };
    let ll = {
        let gui = gui.clone();
        async move {
            layout_receiver.layout_loop(gui).await;
        }
    };
    let pl = {
        let gui = gui.clone();
        async move {
            let write = stdout();
            pin!(write);
            if let Err(e) = paint_receiver.render_loop(gui, write).await {
                eprintln!("Write error {:?}", e);
            }
            std::process::exit(0);
        }
    };
    mem::drop(gui);
    mem::drop(event_sender);
    PriorityRunner::from_iter(vec![el.boxed(), rl.boxed(), ll.boxed(), pl.boxed()].into_iter()).await;
}