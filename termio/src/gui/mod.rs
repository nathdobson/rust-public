use std::{io, mem, thread};
use std::future::Future;
use std::iter::FromIterator;
use std::sync::Arc;
use std::time::Instant;

use async_util::coop::Cancel;
// use async_util::priority::PriorityRunner;
use util::mutrc::MutRc;

use crate::gui::gui::Gui;
use crate::gui::tree::{Tree, TreeReceiver};
use tokio::pin;
use tokio::io::{stdin, AsyncRead, AsyncWrite};
use tokio::io::stdout;
use async_util::futureext::FutureExt;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender, channel};
use async_util::poll::poll_loop;
use async_backtrace::spawn;
use tokio_stream::wrappers::UnboundedReceiverStream;
use crate::gui::div::DivRc;
use std::pin::Pin;
use crate::input::{Event, EventReader, KeyEvent};
use std::io::Error;
use async_util::pipe::unbounded;
use tokio::sync::mpsc;

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

pub struct GuiBuilder {
    cancel: Option<Cancel>,
    tree: Option<(Tree, TreeReceiver)>,
    input: Option<BoxAsyncRead>,
    output: Option<BoxAsyncWrite>,
}

type BoxAsyncRead = Pin<Box<dyn AsyncRead + Send + Sync>>;
type BoxAsyncWrite = Pin<Box<dyn AsyncWrite + Send + Sync>>;

fn piped_stdin() -> BoxAsyncRead {
    let (mut pipe_tx, pipe_rx) = unbounded::pipe();
    thread::spawn(move || {
        if let Err(e) = std::io::copy(&mut std::io::stdin().lock(), &mut pipe_tx) {
            eprintln!("Pipe stdin error {}", e);
        }
    });
    Box::pin(pipe_rx)
}

impl GuiBuilder {
    pub fn new() -> Self {
        GuiBuilder {
            cancel: None,
            tree: None,
            input: None,
            output: None,
        }
    }
    pub fn set_cancel(&mut self, cancel: Cancel) -> &mut Self {
        assert!(self.cancel.is_none());
        self.cancel = Some(cancel);
        self
    }
    pub fn cancel(&mut self) -> Cancel {
        self.cancel.get_or_insert_with(|| Cancel::new()).clone()
    }
    fn set_tree(&mut self) {
        if self.tree.is_none() {
            self.tree = Some(Tree::new(self.cancel()));
        }
    }
    pub fn tree(&mut self) -> Tree {
        self.set_tree();
        self.tree.as_ref().unwrap().0.clone()
    }
    pub fn set_input(&mut self, input: BoxAsyncRead) -> &mut Self {
        self.input = Some(input);
        self
    }
    pub fn set_output(&mut self, output: BoxAsyncWrite) -> &mut Self {
        self.output = Some(output);
        self
    }
    pub fn build(mut self, div: DivRc) -> Gui {
        let (event_sender, event_receiver) = mpsc::channel(1000);
        self.set_tree();
        let cancel = self.cancel();
        let (tree, tree_receiver) = self.tree.unwrap();
        let input = self.input.take().unwrap_or_else(|| Box::pin(piped_stdin()));
        spawn(async move {
            let reader = EventReader::new(input);
            pin!(reader);
            loop {
                match cancel.checked(reader.as_mut().read()).await {
                    Ok(Ok(event)) => {
                        event_sender.send(event).await.ok();
                    }
                    Ok(Err(error)) => {
                        eprintln!("Output error {}", error);
                        break;
                    }
                    Err(_) => break,
                }
            }
        });
        let output = self.output.take().unwrap_or_else(|| Box::pin(stdout()));
        Gui::new(tree, tree_receiver, event_receiver, output, div)
    }
}
