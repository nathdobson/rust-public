use std::{fmt, io, mem, thread};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::str;
use std::sync::Arc;
use std::sync::mpsc::RecvError;
use std::time::{Duration, Instant};

use async_util::coop::Cancel;
use async_util::dirty;
use util::atomic_refcell::AtomicRefCell;
use util::mutrc::MutRc;
use util::pmpsc;

use crate::gui::event::{EventSender, GuiEvent};
use crate::gui::gui::Gui;
use std::io::Write;
use tokio::io::AsyncWrite;
use tokio_stream::StreamExt;
use tokio::io::AsyncWriteExt;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum Dirty {
    Paint,
    Layout,
}

struct TreeInner {
    event_sender: EventSender,
    paint: dirty::Sender,
    layout: dirty::Sender,
    cancel: Cancel,
}

#[derive(Clone, Debug)]
pub struct Tree(Arc<TreeInner>);

pub struct PaintReceiver(dirty::Receiver, Cancel);

pub struct LayoutReceiver(dirty::Receiver, Cancel);

impl Tree {
    pub fn new(cancel: Cancel, event_sender: EventSender) -> (Self, PaintReceiver, LayoutReceiver) {
        let (paint, paint_receiver) = dirty::channel();
        let (layout, layout_receiver) = dirty::channel();
        (Tree(Arc::new(TreeInner { event_sender, paint, layout, cancel: cancel.clone() })),
         PaintReceiver(paint_receiver, cancel.clone()),
         LayoutReceiver(layout_receiver, cancel.clone()))
    }

    pub fn event_sender(&self) -> &EventSender { &self.0.event_sender }

    pub fn mark_dirty(&mut self, dirty: Dirty) {
        match dirty {
            Dirty::Paint => self.0.paint.mark(),
            Dirty::Layout => self.0.layout.mark(),
        }
    }

    pub fn cancel(&self) -> &Cancel {
        &self.0.cancel
    }
}

impl LayoutReceiver {
    pub async fn layout_loop(self, mut gui: MutRc<Gui>) {
        let LayoutReceiver(mut receiver, cancel) = self;
        cancel.checked(async {
            loop {
                gui.write().layout();
                if receiver.next().await.is_none() { break; }
            }
        }).await.ok();
    }
}

impl PaintReceiver {
    pub async fn render_loop(mut self, mut gui: MutRc<Gui>, mut write: Pin<&mut (impl Send + AsyncWrite)>) -> io::Result<()> {
        let mut buffer = vec![];
        loop {
            gui.write().paint_buffer(&mut buffer);
            if !buffer.is_empty() {
                write.write_all(&buffer).await?;
                buffer.clear();
                write.flush().await?;
            }
            if let Ok(Some(_)) = self.1.checked(self.0.next()).await {} else { break; }
        }
        gui.write().paint_buffer(&mut buffer);
        if !buffer.is_empty() {
            write.write_all(&buffer).await?;
            buffer.clear();
            write.flush().await?;
        }
        Ok(())
    }
}

impl Debug for TreeInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeInner").finish()
    }
}

