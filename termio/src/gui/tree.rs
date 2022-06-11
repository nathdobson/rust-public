use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::io::Write;
use std::pin::Pin;
use std::sync::mpsc::RecvError;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{fmt, io, mem, str, thread};

use async_util::coop::Cancel;
use async_util::dirty;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_stream::StreamExt;
use util::atomic_refcell::AtomicRefCell;
use util::mutrc::MutRc;
use util::pmpsc;

// use crate::gui::event::{GuiEvent};
use crate::gui::gui::Gui;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum Dirty {
    Paint,
    Layout,
}

struct TreeInner {
    paint: dirty::Sender,
    layout: dirty::Sender,
    cancel: Cancel,
}

#[derive(Clone, Debug)]
pub struct Tree(Arc<TreeInner>);

pub struct TreeReceiver {
    pub(crate) paint: Option<dirty::Receiver>,
    pub(crate) layout: Option<dirty::Receiver>,
    pub(crate) cancel: Cancel,
}

impl Tree {
    pub fn new(cancel: Cancel) -> (Self, TreeReceiver) {
        let (paint, paint_receiver) = dirty::channel();
        let (layout, layout_receiver) = dirty::channel();
        paint.mark();
        layout.mark();
        (
            Tree(Arc::new(TreeInner {
                paint,
                layout,
                cancel: cancel.clone(),
            })),
            TreeReceiver {
                paint: Some(paint_receiver),
                layout: Some(layout_receiver),
                cancel: cancel.clone(),
            },
        )
    }

    pub fn mark_dirty(&mut self, dirty: Dirty) {
        match dirty {
            Dirty::Paint => self.0.paint.mark(),
            Dirty::Layout => self.0.layout.mark(),
        }
    }

    pub fn cancel(&self) -> &Cancel { &self.0.cancel }
}

impl Debug for TreeInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result { f.debug_struct("TreeInner").finish() }
}
