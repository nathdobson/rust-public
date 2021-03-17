use std::time::Instant;
use crate::gui::tree::Tree;
use util::mutrc::MutRc;
use crate::gui::gui::Gui;
use std::{mem, io};
use async_std::io::{stdout, stdin};
use crate::gui::event::event_loop;
use futures::{join, FutureExt};
use futures::pin_mut;
use crate::gui::event::read_loop;
use std::future::Future;
use futures::executor::{ThreadPool, block_on};
use std::sync::Arc;
use std::iter::FromIterator;
use async_util::priority::PriorityRunner;
use async_util::cancel::Cancel;

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

pub fn run_local(gui: impl Send + FnOnce(Tree) -> MutRc<Gui>) {
    fn run_local_impl(gui: impl Send + FnOnce(Tree) -> MutRc<Gui>) -> impl Future<Output=()> + Send {
        async {
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
                    pin_mut!(write);
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
    }
    block_on(run_local_impl(gui));
}