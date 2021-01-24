use crate::gui::gui::{Gui};
use std::{thread, mem, fmt, io};
use std::fmt::{Formatter, Debug};
use std::time::{Duration, Instant};
use util::pmpsc;
use std::sync::mpsc::RecvError;
use std::any::Any;
use util::atomic_refcell::AtomicRefCell;
use util::any::Upcast;
use crate::gui::event::{GuiEvent, EventSender};
use util::mutrc::MutRc;
use async_util::{Condvar, Mutex, CondvarWeak};
use std::collections::HashSet;
use std::sync::Arc;
use std::pin::Pin;
use futures::{AsyncWrite, AsyncWriteExt};
use std::str;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum Dirty {
    Paint,
    Layout,
    Close,
}

struct TreeInner {
    event_sender: EventSender,
    dirty: MutRc<HashSet<Dirty>>,
    when_dirty: Condvar,
}

#[derive(Clone, Debug)]
pub struct Tree(Arc<TreeInner>);

impl Tree {
    pub fn new(event_sender: EventSender) -> Self {
        Tree(Arc::new(TreeInner {
            event_sender,
            dirty: MutRc::new([Dirty::Paint, Dirty::Layout].iter().cloned().collect()),
            when_dirty: Condvar::new(),
        }))
    }

    pub fn event_sender(&self) -> &EventSender { &self.0.event_sender }

    pub fn mark_dirty(&mut self, dirty: Dirty) {
        self.0.dirty.borrow_mut().insert(dirty);
        self.0.when_dirty.notify_one();
    }

    pub async fn render_loop(
        self,
        mut gui: MutRc<Gui>,
        mut write: Pin<&mut (impl  Send + AsyncWrite)>,
    ) -> io::Result<()> {
        let mutex = self.0.event_sender.mutex().clone();
        let mut dirty = self.0.dirty.clone();
        let when_dirty = self.0.when_dirty.downgrade();
        mem::drop(self);
        let mut buffer = vec![];
        let mut lock = mutex.lock().await;
        loop {
            let mut close = false;
            {
                if dirty.write().remove(&Dirty::Layout) {
                    dirty.write().insert(Dirty::Paint);
                    gui.write().layout();
                }
                if dirty.write().remove(&Dirty::Paint) {
                    gui.write().paint_buffer(&mut buffer);
                }
                if dirty.write().remove(&Dirty::Close) {
                    close = true;
                }
            }
            if !buffer.is_empty() {
                mem::drop(lock);
                write.write_all(&buffer).await?;
                buffer.clear();
                write.flush().await?;
                lock = mutex.lock().await;
            }
            if close {
                break;
            }
            match when_dirty.wait(lock).await {
                Ok(l) => { lock = l; }
                Err(e) => break,
            }
        }
        Ok(())
    }
}


impl Debug for TreeInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeInner").finish()
    }
}

#[must_use]
pub struct DirtyReceiver {
    mutex: Mutex,
    state: MutRc<HashSet<Dirty>>,
    condvar: CondvarWeak,
}
