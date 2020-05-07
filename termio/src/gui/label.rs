use util::shared::{Shared, SharedMut};
use crate::canvas::Canvas;
use crate::input::Event;
use itertools::Itertools;
use crate::color::Color;
use crate::output::{Background, Foreground};
use std::ops;
use crate::gui::node::{NodeHeader, NodeImpl, Node};
use crate::gui::node::NodeExt;
use crate::screen::LineSetting;
use crate::gui::{InputEvent, OutputEvent};
use std::sync::Arc;

#[derive(Debug)]
pub struct Label {
    header: NodeHeader,
    text: String,
    pub size: (isize, isize),
}

impl NodeImpl for Label {
    fn header(&self) -> &NodeHeader {
        &self.header
    }

    fn header_mut(&mut self) -> &mut NodeHeader {
        &mut self.header
    }

    fn paint(&self, mut w: Canvas) {
        let lines =
            self.text
                .split('\n')
                .rev()
                .filter(|x| !x.is_empty())
                .take(self.size.1 as usize)
                .collect_vec()
                .into_iter()
                .rev()
                .enumerate();
        for (y, line) in lines {
            w.draw((0, y as isize), line);
        }
    }

    fn handle(&mut self, event: &InputEvent,output:&mut Vec<Arc<dyn OutputEvent>>) {}

    fn size(&self) -> (isize, isize) {
        self.size
    }
}


impl Label {
    pub fn new(text: String, size: (isize, isize)) -> Node<Label> {
        Self::new_internal(|header| Label {
            header,
            text,
            size,
        })
    }
    pub fn text_mut(&mut self) -> &mut String {
        self.header.mark_dirty();
        &mut self.text
    }
}