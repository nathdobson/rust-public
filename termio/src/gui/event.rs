//use util::any::Upcast;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::{Debug, Formatter};
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::sync::mpsc::RecvError;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{fmt, io, mem, str, sync, thread};

use async_util::spawn::Spawn;
use libc::close;
use tokio::io::AsyncRead;
use tokio::pin;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;
use tokio_stream::Stream;
use util::atomic_refcell::AtomicRefCell;
use util::mutrc::MutRc;
use util::shared::Shared;
use util::{lossy, pmpsc};

use crate::gui::div::{Div, DivImpl, DivRc};
use crate::gui::gui::Gui;
use crate::gui::tree::Tree;
use crate::input::{Event, EventReader};

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
    pub fn new(f: impl 'static + Fn() + Send + Sync) -> Self { BoxFnMut(Box::new(f)) }
    pub fn run(&mut self) { (self.0)() }
    pub fn new_channel() -> (BoxFnMut, UnboundedReceiver<()>) {
        let (tx, rx) = unbounded_channel();
        (
            BoxFnMut::new(move || {
                tx.send(()).ok();
            }),
            rx,
        )
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
