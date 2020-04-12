use termio::input::Event;
use termio::output::{SafeWrite, MoveWindow, ResizeWindow, AllMotionTrackingEnable, FocusTrackingEnable, ReportWindowPosition, ReportWindowSize};
use crate::{NetcatHandler, NetcatPeer};
use std::thread;
use std::time::Duration;
use util::object::Object;

pub struct DemoHandler {
}

impl DemoHandler {
    pub fn new() -> Self {
        DemoHandler {  }
    }
}

impl NetcatHandler for DemoHandler {
    fn add_peer(&mut self, _: &Object, mut peer: NetcatPeer) {
        write!(peer, "{}", AllMotionTrackingEnable);
        write!(peer, "{}", FocusTrackingEnable);
        write!(peer, "{}", ReportWindowPosition);
        write!(peer, "{}", ReportWindowSize);
        peer.flush();
        thread::spawn(move || {
            for i in 0.. {
                let theta = (i as f32) / 100.0;
                let width = (((theta.cos().abs()) * 1440.0) as usize).max(200);
                let height = ((theta.sin().abs()) * 800.0) as usize;
                let x = (2880 / 2 - width) / 2;
                let y = (1800 / 2 - height) / 2;
                thread::sleep(Duration::from_millis(10));
                write!(peer, "{}{}", MoveWindow(x, y), ResizeWindow(width, height));
                peer.flush();
            }
        });
    }

    fn remove_peer(&mut self, _: &Object) {}

    fn handle_event(&mut self, _: &Object, _: &Event) {
        //println!("{:?}", event);
    }
}
