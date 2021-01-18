use util::any::Upcast;
use std::any::Any;
use crate::gui::node::Node;
use crate::gui::view::{View, ViewImpl};

pub trait UpcastController {
    fn upcast_controller(&self) -> &dyn Controller;
    fn upcast_controller_mut(&mut self) -> &mut dyn Controller;
}

impl<T: Controller> UpcastController for T {
    fn upcast_controller(&self) -> &dyn Controller { self }
    fn upcast_controller_mut(&mut self) -> &mut dyn Controller { self }
}

pub trait Controller: 'static + Send + Sync + Upcast<dyn Any> + UpcastController {}

pub trait ControllerExt: Controller {
    fn descendant_mut<'a, T: ViewImpl>(&'a mut self, node: Node<T>) -> &'a mut View<T> {
        let v2: &'a mut View = node.descend_mut(self.upcast_controller_mut());
        v2.downcast_view_mut()
    }
}

impl<T: Controller + ?Sized> ControllerExt for T {}
