#![feature(never_type, arbitrary_self_types)]
#![feature(box_syntax)]
#![allow(unused_imports)]

use termio::gui::gui::{Gui, InputEvent};
use termio::gui::button::Button;
use std::sync::{Arc, mpsc, Mutex};
use termio::input::{EventReader, Event, KeyEvent};
use std::io::{stdin, stdout, Write};
use std::error::Error;
use std::{mem, thread, process};
use util::grid::Grid;
use termio::gui::label::Label;
use termio::string::{StyleFormatExt, StyleString};
use util::any::{Upcast};
use std::any::Any;
use std::ops::Deref;
use termio::screen::Style;
use termio::color::Color;
use std::time::{Instant, Duration};
use std::time;
use termio::gui::tree::{Tree, Dirty};
use termio::gui::layout::{Constraint, Layout};
use util::lossy;
use std::str;
use termio::gui::event::{GuiEvent, SharedGuiEvent};
use termio::gui::{event, run_local};
use termio::gui::div::{Div, DivImpl, DivRc, DivWeak};
use util::atomic_refcell::AtomicRefCell;
use util::mutrc::MutRc;
use termio::canvas::Canvas;
use rand::{thread_rng, Rng};
use rand::seq::SliceRandom;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::AtomicUsize;

#[derive(Debug)]
struct Example {
    time: AtomicUsize,
}

impl Example {
    fn new(tree: Tree) -> DivRc<Self> {
        let mut result = DivRc::new(tree.clone(), Example { time: AtomicUsize::new(0) });
        result.write().animate();
        result
    }
    fn animate(self: &mut Div<Self>) {
        self.mark_dirty(Dirty::Paint);
        self.event_sender()
            .run_with_delay(
                Duration::from_millis(100),
                self.new_event(|this| this.animate()))
    }
    fn new_gui(tree: Tree) -> MutRc<Gui> {
        let mut gui = Gui::new(tree.clone(), Example::new(tree));
        gui.set_background(Style {
            background: Color::Gray24(23),
            foreground: Color::Gray24(0),
            ..Style::default()
        });
        MutRc::new(gui)
    }
}

impl DivImpl for Example {
    fn layout_impl(self: &mut Div<Self>, constraint: &Constraint) -> Layout {
        Layout { size: constraint.max_size.unwrap(), line_settings: Default::default() }
    }
    fn self_paint_below(self: &Div<Self>, mut canvas: Canvas) {
        let time = self.time.fetch_add(1, Relaxed);
        let colors = [
            Color::RGB666(5, 0, 0),
            Color::RGB666(5, 3, 2),
            Color::RGB666(5, 5, 0),
            Color::RGB666(0, 5, 0),
            Color::RGB666(0, 0, 5),
            Color::RGB666(3, 0, 5)
        ];
        for x in 0..self.size().0 {
            canvas.style.background = colors[((100000000 - time + x as usize) % colors.len())];
            for y in 0..self.size().1 {
                canvas.draw((x, y), &" ");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    run_local(|tree| Example::new_gui(tree)).await;
}
