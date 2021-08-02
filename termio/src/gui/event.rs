//use util::any::Upcast;
use std::{mem, sync, thread};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::fmt;
use std::fmt::Formatter;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::str;
use std::sync::Arc;
use std::sync::mpsc::RecvError;
use std::task::{Context, Poll};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::pin;
use libc::close;
use std::future::poll_fn;
use async_util::spawn::Spawn;

use util::{lossy, pmpsc};
use util::atomic_refcell::AtomicRefCell;
use util::mutrc::MutRc;

use crate::gui::div::{Div, DivImpl, DivRc};
use crate::gui::gui::Gui;
use crate::gui::tree::Tree;
use crate::input::{EventReader, Event};
use tokio::time::sleep;
use tokio_stream::Stream;
use tokio::io::AsyncRead;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel, UnboundedReceiver};
use util::shared::Shared;

// #[must_use]
// pub struct GuiEvent(Box<dyn FnOnce() + Send + Sync>);

#[must_use]
pub struct BoxFnMut(Box<dyn FnMut() + Send + Sync>);

// impl GuiEvent {
//     pub fn new(f: impl 'static + Send + Sync + FnOnce()) -> Self {
//         GuiEvent(Box::new(f))
//     }
// }

impl BoxFnMut {
    pub fn new(f: impl 'static + Fn() + Send + Sync) -> Self {
        BoxFnMut(Box::new(f))
    }
    pub fn run(&mut self) {
        (self.0)()
    }
    pub fn new_channel() -> (BoxFnMut, UnboundedReceiver<()>) {
        let (tx, rx) = unbounded_channel();
        (BoxFnMut::new(move || { tx.send(()).ok(); }), rx)
    }
}

// impl GuiEvent {
//     fn run(self) {
//         (self.0)();
//     }
// }

impl Debug for BoxFnMut {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedGuiEvent").finish()
    }
}

// impl Debug for GuiEvent {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         f.debug_struct("GuiEvent").finish()
//     }
// }