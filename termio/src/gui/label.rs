use util::shared::{Header, HasHeader, Shared};
use crate::gui::{NodeHeader, IsNode, NodeEvent};
use crate::canvas::Canvas;
use crate::input::Event;
use crate::gui::button::Button;
use crate::gui::Node;
use itertools::Itertools;

#[derive(Debug)]
pub struct Label {
    header: Header<Label, NodeHeader>,
    pub text: String,
    pub size: (isize, isize),
}

impl HasHeader<NodeHeader> for Label {
    fn shared_header(&self) -> &Header<Self, NodeHeader> { &self.header }
    fn shared_header_mut(&mut self) -> &mut Header<Self, NodeHeader> { &mut self.header }
}

impl IsNode for Label {
    fn paint(&self, w: &mut Canvas) {
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

    fn handle_event(&mut self, event: &Event) -> Option<NodeEvent> {
        None
    }

    fn size(&self) -> (isize, isize) {
        self.size
    }
}


impl Label {
    pub fn new(text: String, size: (isize, isize)) -> Node<Label> {
        Header::new_shared(Label {
            header: Header::new_header(NodeHeader::new()),
            text,
            size,
        })
    }
}