use std::sync::{Arc, Weak};
use by_address::ByAddress;
use util::shared::{Shared, WkShared};
use atomic_refcell::{AtomicRefCell, AtomicRef};
use std::marker::PhantomData;
use util::any::{Upcast, AnyExt};
use std::any::Any;
use std::ops::{CoerceUnsized, Deref};
use std::{slice, fmt};
use crate::gui::view::{View, ViewImpl};
use util::rect::Rect;
use crate::gui::tree::{Tree};
use std::collections::HashMap;
use crate::screen::LineSetting;
use std::fmt::Debug;
use serde::export::Formatter;
use std::cmp::Ordering;
use std::ops::DerefMut;
use crate::gui::gui::Gui;
use crate::gui::event::{SharedGuiEvent, GuiEvent, EventSender};

enum NodeParent {
    IsChild {
        id: Node,
        get_ref: Box<dyn Send + Sync + for<'a> Fn(&'a View) -> &'a View>,
        get_mut: Box<dyn Send + Sync + for<'a> Fn(&'a mut View) -> &'a mut View>,
    },
    IsRoot {
        gui: Weak<AtomicRefCell<Gui>>,
        get_ref: Box<dyn Send + Sync + for<'a> Fn(&'a dyn Controller) -> &'a Gui>,
        get_mut: Box<dyn Send + Sync + for<'a> Fn(&'a mut dyn Controller) -> &'a mut Gui>,
    },
}

#[derive(Debug)]
struct NodeState {
    children: Vec<Shared<NodeInner>>,
}

#[derive(Debug)]
struct NodeInner {
    parent: NodeParent,
    context: Tree,
    state: AtomicRefCell<NodeState>,
}

#[derive(Debug)]
pub struct NodeStrong<T: ViewImpl + ?Sized = dyn ViewImpl>(Shared<NodeInner>, PhantomData<T>);

#[derive(Debug)]
pub struct Node<T: ViewImpl + ?Sized = dyn ViewImpl>(WkShared<NodeInner>, PhantomData<T>);

impl NodeState {
    fn new() -> NodeState {
        NodeState { children: vec![] }
    }
}

impl NodeStrong {
    pub fn downcast_node<T: ViewImpl>(self) -> NodeStrong<T> {
        NodeStrong(self.0, PhantomData)
    }
}

impl<T: ViewImpl + ?Sized> NodeStrong<T> {
    pub fn root<C, MD, GR, GM>(
        event_sender: EventSender,
        mark_dirty: MD,
        get_ref: GR,
        get_mut: GM,
    ) -> Self
        where C: Controller,
              MD: 'static + Send + Sync + Fn(),
              GR: 'static + Send + Sync + Fn(&C) -> &Gui,
              GM: 'static + Send + Sync + Fn(&mut C) -> &mut Gui
    {
        let context = Tree::new(event_sender, Box::new(mark_dirty));
        NodeStrong(Shared::new(NodeInner {
            parent: NodeParent::IsRoot {
                gui: todo!(),
                get_ref: Box::new(move |controller| { get_ref(controller.upcast().downcast_ref().unwrap()) }),
                get_mut: Box::new(move |controller| { get_mut(controller.upcast_mut().downcast_mut().unwrap()) }),
            },
            context,
            state: AtomicRefCell::new(NodeState::new()),
        }), PhantomData)
    }
    pub fn downgrade(&self) -> Node<T> {
        Node(self.0.downgrade(), PhantomData)
    }

    pub fn upcast_node(self) -> NodeStrong {
        NodeStrong(self.0, PhantomData)
    }

    pub fn context(&self) -> &Tree {
        &self.0.context
    }
}

impl<T: ViewImpl> NodeStrong<T> {
    pub fn child<T2: ViewImpl>(
        &self,
        get_ref: impl 'static + Send + Sync + Fn(&T) -> &View<T2>,
        get_mut: impl 'static + Send + Sync + Fn(&mut T) -> &mut View<T2>)
        -> NodeStrong<T2> {
        let child = NodeStrong(Shared::new(NodeInner {
            parent: NodeParent::IsChild {
                id: self.downgrade().upcast_node(),
                get_ref: Box::new(move |model|
                    get_ref(model.deref().downcast_ref_result().unwrap()) as &View),
                get_mut: Box::new(move |model|
                    get_mut(model.deref_mut().downcast_mut_result().unwrap()) as &mut View),
            },
            context: self.0.context.clone(),
            state: AtomicRefCell::new(NodeState::new()),
        }), PhantomData);
        self.0.state.borrow_mut().children.push(child.0.clone());
        child
    }
    pub fn new_event(&self, cb: impl FnOnce(&mut View<T>) + Send + Sync + 'static) -> GuiEvent where T: Sized {
        self.downgrade().new_event(cb)
    }
    pub fn new_shared_event(&self, cb: impl Fn(&mut View<T>) + Send + Sync + 'static) -> SharedGuiEvent where T: Sized {
        self.downgrade().new_shared_event(cb)
    }
    pub fn new_root_shared_event(&self, cb: impl Fn(&mut Gui<T>) + Send + Sync + 'static) -> SharedGuiEvent where T: Sized {
        self.downgrade().new_root_shared_event(cb)
    }
}

impl Node<dyn ViewImpl> {
    pub fn downcast_node<T: ViewImpl + ?Sized>(self) -> Node<T> {
        Node(self.0, PhantomData)
    }
}

impl<T: ViewImpl + ?Sized> Node<T> {
    pub fn upcast_node(self) -> Node {
        Node(self.0, PhantomData)
    }
    pub fn child_mut<'a, 'b>(&'a self, view: &'b mut View) -> &'b mut View {
        let inner = self.0.upgrade().unwrap();
        match &inner.parent {
            NodeParent::IsChild { get_mut, .. } => {
                get_mut(view)
            }
            NodeParent::IsRoot { .. } => panic!()
        }
    }

    pub fn root_mut<'a, 'b>(&'a self, controller: &'b mut dyn Controller) -> &'b mut Gui {
        let inner = self.0.upgrade().unwrap();
        match &inner.parent {
            NodeParent::IsChild {
                id,
                get_ref,
                get_mut
            } => {
                id.root_mut(controller)
            }
            NodeParent::IsRoot {
                get_ref,
                get_mut, ..
            } => {
                get_mut(controller)
            }
        }
    }

    pub fn descend_mut<'a, 'b>(&'a self, controller: &'b mut dyn Controller) -> &'b mut View {
        let inner = self.0.upgrade().unwrap();
        match &inner.parent {
            NodeParent::IsChild {
                id,
                get_ref,
                get_mut
            } => {
                get_mut(id.descend_mut(controller))
            }
            NodeParent::IsRoot {
                get_ref,
                get_mut, ..
            } => {
                get_mut(controller).root_mut()
            }
        }
    }

    pub fn new_event(self, cb: impl FnOnce(&mut View<T>) + Send + Sync + 'static) -> GuiEvent where T: Sized {
        //GuiEvent::new_dyn(|gui| cb(gui.descendant_mut(self)))
        todo!()
    }
    pub fn new_shared_event(self, cb: impl Fn(&mut View<T>) + Send + Sync + 'static) -> SharedGuiEvent where T: Sized {
        //SharedGuiEvent::new_dyn(move |gui| cb(gui.descendant_mut(self.clone())))
        todo!()
    }
    pub fn new_root_shared_event(self, cb: impl Fn(&mut Gui<T>) + Send + Sync + 'static) -> SharedGuiEvent where T: Sized {
        //SharedGuiEvent::new_dyn(move |gui| cb(gui.root_mut(self.clone())))
        todo!()
    }
}

impl<T: ViewImpl + ?Sized> Clone for Node<T> {
    fn clone(&self) -> Self {
        Node(self.0.clone(), PhantomData)
    }
}

impl<T: ViewImpl + ?Sized> Clone for NodeStrong<T> {
    fn clone(&self) -> Self {
        NodeStrong(self.0.clone(), PhantomData)
    }
}

pub struct Children<'a> {
    view: &'a View,
    state: AtomicRef<'a, NodeState>,
}

pub struct ChildrenIter<'a> {
    view: &'a View,
    iter: slice::Iter<'a, Shared<NodeInner>>,
}

impl<'a> Children<'a> {
    pub fn new(view: &'a View) -> Self {
        Children { view, state: view.node_strong().0.state.borrow() }
    }
}

impl<'a> IntoIterator for &'a Children<'a> {
    type Item = &'a View;
    type IntoIter = ChildrenIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ChildrenIter { view: self.view, iter: self.state.children.iter() }
    }
}

impl<'a> Iterator for ChildrenIter<'a> {
    type Item = &'a View;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            match &next.parent {
                NodeParent::IsChild { get_ref, .. } => {
                    Some(get_ref(self.view))
                }
                NodeParent::IsRoot { .. } => panic!("Cannot find root")
            }
        } else {
            None
        }
    }
}

pub struct ChildrenMut {
    node: Shared<NodeInner>,
}

pub struct ChildrenMutBorrow<'a> {
    state: AtomicRef<'a, NodeState>,
}

pub struct ChildrenMutIter<'a> {
    iter: slice::Iter<'a, Shared<NodeInner>>,
}

impl ChildrenMut {
    pub fn into_iterable<'a>(&'a self) -> ChildrenMutBorrow<'a> {
        ChildrenMutBorrow { state: self.node.state.borrow() }
    }
}

impl ChildrenMut {
    pub fn new(view: &View) -> Self {
        ChildrenMut { node: view.node_strong().0.clone() }
    }
}

impl<'a> IntoIterator for &'a ChildrenMutBorrow<'a> {
    type Item = &'a (dyn Send + Sync + for<'b> Fn(&'b mut View) -> &'b mut View);
    type IntoIter = ChildrenMutIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ChildrenMutIter { iter: self.state.children.iter() }
    }
}

impl<'a> Iterator for ChildrenMutIter<'a> {
    type Item = &'a (dyn Send + Sync + for<'b> Fn(&'b mut View) -> &'b mut View);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            match &next.parent {
                NodeParent::IsChild { get_mut, .. } => {
                    Some(get_mut)
                }
                NodeParent::IsRoot { .. } => panic!("Cannot find root")
            }
        } else {
            None
        }
    }
}

impl Debug for NodeParent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeParent")
            .finish()
    }
}

impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
