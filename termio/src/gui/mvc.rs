use std::sync::{Arc, Weak};
use by_address::ByAddress;
use util::shared::{Shared, WkShared};
use atomic_refcell::{AtomicRefCell, AtomicRef};
use std::marker::PhantomData;
use util::any::{Upcast, AnyExt};
use std::any::Any;
use std::ops::{CoerceUnsized, Deref};
use std::slice;

#[derive(Clone)]
pub struct Context {}

pub trait View: 'static + Upcast<dyn Any> {
    fn layout_self(&self);
    fn node(&self) -> &Node;
}

struct NodeParent {
    id: NodeWeak,
    get_ref: Box<dyn for<'a> Fn(&'a dyn View) -> &'a dyn View>,
    get_mut: Box<dyn for<'a> Fn(&'a mut dyn View) -> &'a mut dyn View>,
}

struct NodeState {
    children: Vec<Node>,

}

struct NodeInner {
    parent: Option<NodeParent>,
    context: Context,
    state: AtomicRefCell<NodeState>,
}

pub struct Node<T: View + ?Sized = dyn View>(Shared<NodeInner>, PhantomData<T>);

pub struct NodeWeak<T: View + ?Sized = dyn View>(WkShared<NodeInner>, PhantomData<T>);

impl NodeState {
    fn new() -> NodeState {
        NodeState { children: vec![] }
    }
}

impl<T: View + ?Sized> Node<T> {
    pub fn root(context: Context) -> Self {
        Node(Shared::new(NodeInner {
            parent: None,
            context,
            state: AtomicRefCell::new(NodeState::new()),
        }), PhantomData)
    }
    pub fn downgrade(&self) -> NodeWeak<T> {
        NodeWeak(self.0.downgrade(), PhantomData)
    }
    pub fn upcast_view(self) -> Node {
        Node(self.0, PhantomData)
    }
}

impl<T: View> Node<T> {
    pub fn child<T2: View>(&self,
                            get_ref: impl 'static + Fn(&T) -> &T2,
                            get_mut: impl 'static + Fn(&mut T) -> &mut T2)
                            -> Node<T2> {
        let child = Node(Shared::new(NodeInner {
            parent: Some(NodeParent {
                id: self.downgrade().upcast_view(),
                get_ref: Box::new(move |model|
                    get_ref(model.downcast_ref_result().unwrap()) as &dyn View),
                get_mut: Box::new(move |model|
                    get_mut(model.downcast_mut_result().unwrap()) as &mut dyn View),
            }),
            context: self.0.context.clone(),
            state: AtomicRefCell::new(NodeState::new()),
        }), PhantomData);
        self.0.state.borrow_mut().children.push(child.clone().upcast_view());
        child
    }
}

impl<T: View + ?Sized> NodeWeak<T> {
    pub fn upgrade(&self) -> Node<T> {
        Node(self.0.upgrade().unwrap(), PhantomData)
    }
    pub fn upcast_view(self) -> NodeWeak {
        NodeWeak(self.0, PhantomData)
    }
}

impl<T: View + ?Sized> Clone for Node<T> {
    fn clone(&self) -> Self {
        Node(self.0.clone(), PhantomData)
    }
}

impl<T: View + ?Sized> Clone for NodeWeak<T> {
    fn clone(&self) -> Self {
        NodeWeak(self.0.clone(), PhantomData)
    }
}


trait ViewExt: View + Sized {
    fn children<'a>(&'a self) -> Children<'a> {
        Children { node: self, state: self.node().0.state.borrow() }
    }
    fn children_mut<'a>(&'a self) -> ChildrenMut {
        ChildrenMut { node: self.node().clone() }
    }
}

impl<T: View> ViewExt for T {}

struct Children<'a> {
    node: &'a dyn View,
    state: AtomicRef<'a, NodeState>,
}

struct ChildrenIter<'a> {
    node: &'a dyn View,
    iter: slice::Iter<'a, Node>,
}

impl<'a> IntoIterator for &'a Children<'a> {
    type Item = &'a dyn View;
    type IntoIter = ChildrenIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ChildrenIter { node: self.node, iter: self.state.children.iter() }
    }
}

impl<'a> Iterator for ChildrenIter<'a> {
    type Item = &'a dyn View;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            let get_ref: &Box<dyn Fn(&'a dyn View) -> &'a dyn View> = &next.0.parent.as_ref().unwrap().get_ref;
            let node: &'a dyn View = self.node;
            Some((get_ref)(node))
        } else {
            None
        }
    }
}

struct ChildrenMut {
    node: Node,
}

struct ChildrenMutBorrow<'a> {
    state: AtomicRef<'a, NodeState>,
}

struct ChildrenMutIter<'a> {
    iter: slice::Iter<'a, Node>,
}

impl ChildrenMut {
    fn into_iterable<'a>(&'a self) -> ChildrenMutBorrow<'a> {
        ChildrenMutBorrow { state: self.node.0.state.borrow() }
    }
}

impl<'a> IntoIterator for &'a ChildrenMutBorrow<'a> {
    type Item = &'a dyn for<'b> Fn(&'b mut dyn View) -> &'b mut dyn View;
    type IntoIter = ChildrenMutIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ChildrenMutIter { iter: self.state.children.iter() }
    }
}

impl<'a> Iterator for ChildrenMutIter<'a> {
    type Item = &'a dyn for<'b> Fn(&'b mut dyn View) -> &'b mut dyn View;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|child| child.0.parent.as_ref().unwrap().get_mut.deref())
    }
}