use std::sync::{Mutex, mpsc, Arc};
use crate::gui::gui::{Gui};
use std::{thread, mem, fmt};
use std::fmt::{Formatter, Debug};
use std::time::{Duration, Instant};
use util::pmpsc;
use std::sync::mpsc::RecvError;
use std::any::Any;
use atomic_refcell::AtomicRefCell;
use crate::gui::node::Node;
use util::any::Upcast;
use crate::gui::view::{ViewImpl, View};
use crate::gui::event::{Priority, GuiEvent, EventSender};


struct TreeInner {
    event_sender: EventSender,
    mark_dirty: Box<dyn Fn() + Send + Sync>,
}


#[derive(Clone, Debug)]
pub struct Tree(Arc<TreeInner>);

impl Tree {
    pub fn new(event_sender: EventSender, mark_dirty: Box<dyn Fn() + Send + Sync>) -> Self {
        Tree(Arc::new(TreeInner { event_sender, mark_dirty }))
    }

    pub fn event_sender(&self)->&EventSender{&self.0.event_sender}

    pub fn mark_dirty(&self) {
        (self.0.mark_dirty)()
    }
}



impl Debug for TreeInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeInner").finish()
    }
}