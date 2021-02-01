use std::time::Instant;
use crate::gui::tree::Tree;
use util::mutrc::MutRc;
use crate::gui::gui::Gui;
use std::{mem, io};
use async_std::io::{stdout, stdin};
use crate::gui::event::event_loop;
use async_util::Mutex;
use futures::join;
use futures::pin_mut;
use crate::gui::event::read_loop;
use std::future::Future;
use futures::executor::{ThreadPool, block_on};
use std::sync::Arc;

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

pub fn run_local(gui: impl Send + FnOnce(Tree) -> MutRc<Gui>) {
    fn run_local_impl(gui: impl Send + FnOnce(Tree) -> MutRc<Gui>) -> impl Future<Output=()> + Send {
        async {
            let mutex = Mutex::new();
            let exec = Arc::new(ThreadPool::new().unwrap());
            let (event_sender, el) = event_loop(mutex.clone(), exec);
            let tree = Tree::new(event_sender.clone());
            let gui = gui(tree.clone());
            let rl = read_loop(event_sender.clone(), gui.clone(), stdin());

            let wl = async move {
                let write = stdout();
                pin_mut!(write);
                tree.clone().render_loop(gui, write).await?;
                std::process::exit(0) as io::Result<!>
            };
            mem::drop(mutex);
            mem::drop(event_sender);
            let (r, w, e) = join!(rl, wl, el);
            w.unwrap();
            r.unwrap();
        }
    }
    block_on(run_local_impl(gui));
}