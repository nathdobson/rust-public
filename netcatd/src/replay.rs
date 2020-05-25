use std::path::Path;
use termio::input::{Event, KeyEvent};
use std::collections::{HashSet, HashMap, BTreeMap, BTreeSet};
use std::fs::File;
use std::{fs, io, mem};
use std::io::{BufReader, BufRead, Write};
use std::error::Error;
use rand_xorshift::XorShiftRng;
use util::{completable, Name};

extern crate itertools;

use itertools::Itertools;
use std::sync::{Mutex, Arc};
use rand::{SeedableRng, RngCore};
use util::rng::BoxRng;
use crate::{Handler, Renderer, Timer, TimerCallback, timer};
use util::io::SafeWrite;
use termio::screen::Screen;

#[derive(Serialize, Deserialize, Debug)]
enum EventType {
    Add,
    Shutdown,
    Close,
    Input(Event),
}

#[derive(Serialize, Deserialize, Debug)]
struct ReplayEvent {
    username: String,
    event: EventType,
}

pub struct ReplayHandlerBuilder {
    input: Vec<ReplayEvent>,
    output: File,
    timer: Arc<dyn Timer>,
    //renderer: Arc<dyn Renderer>,
}

pub struct ReplayHandler {
    inner: Box<dyn Handler>,
    output: File,
}

impl ReplayHandlerBuilder {
    pub fn new(directory: &Path, _: BoxRng, timer: Arc<dyn Timer>, _: Arc<dyn Renderer>) -> Result<Self, Box<dyn Error>> {
        let mut input_files: Vec<isize> =
            fs::read_dir(&directory)?
                .map_results(|e| e.file_name().to_str()?.parse::<isize>().ok())
                .filter_map(Result::transpose)
                .collect::<io::Result<Vec<_>>>()?;

        input_files.sort();
        let mut input = Vec::<ReplayEvent>::new();
        for input_file in input_files.iter() {
            for line in BufReader::new(File::open(directory.join(input_file.to_string()))?).lines() {
                let line = line?;
                let event: ReplayEvent = serde_json::from_str(&line)?;
                input.push(event);
            }
        }
        let output = input_files.iter().cloned().max().unwrap_or(-1) + 1;
        let output = File::create(directory.join(format!("{}", output)))?;
        Ok(ReplayHandlerBuilder { input, output, timer })
    }
    pub fn rng(&self) -> BoxRng {
        Box::new(XorShiftRng::from_seed(*b"www.xkcd.com/221"))
    }
    pub fn timer(&self) -> Arc<dyn Timer> {
        self.timer.clone()
    }
    pub fn build(self, mut inner: Box<dyn Handler>) -> ReplayHandler {
        let mut open = HashSet::<Name>::new();
        let mut half_open = HashSet::<Name>::new();
        for event in self.input.iter() {
            let username = Arc::new(event.username.clone());
            match event.event {
                EventType::Add => {
                    inner.peer_add(&username);
                    open.insert(username.clone());
                    half_open.insert(username);
                }
                EventType::Shutdown => {
                    inner.peer_shutdown(&username);
                    open.remove(&username);
                }
                EventType::Close => {
                    inner.peer_close(&username);
                    half_open.remove(&username);
                }
                EventType::Input(event) =>
                    inner.peer_event(&username, &event),
            }
        }
        let mut result = ReplayHandler {
            inner,
            output: self.output,
        };
        for username in open {
            result.peer_shutdown(&username);
        }
        for username in half_open {
            result.peer_close(&username);
        }
        result
    }
}

impl ReplayHandler {
    fn write(&mut self, event: ReplayEvent) {
        writeln!(self.output, "{}", serde_json::to_string(&event).unwrap()).unwrap();
        self.output.flush().unwrap();
    }
}

impl Handler for ReplayHandler {
    fn peer_add(&mut self, username: &Name) {
        self.inner.peer_add(username);
        self.write(ReplayEvent {
            username: (**username).clone(),
            event: EventType::Add,
        });
    }

    fn peer_shutdown(&mut self, username: &Name) {
        self.inner.peer_shutdown(username);
        self.write(ReplayEvent {
            username: (**username).clone(),
            event: EventType::Shutdown,
        });
    }

    fn peer_close(&mut self, username: &Name) {
        self.inner.peer_close(username);
        self.write(ReplayEvent {
            username: (**username).clone(),
            event: EventType::Close,
        });
    }

    fn peer_event(&mut self, username: &Name, event: &Event) {
        self.inner.peer_event(username, event);
        if event != &Event::KeyEvent(KeyEvent::typed('q').control()) {
            self.write(ReplayEvent {
                username: (**username).clone(),
                event: EventType::Input(*event),
            });
        }
    }

    fn peer_render(&mut self, username: &Arc<String>, output: &mut Vec<u8>) {
        self.inner.peer_render(username, output);
    }
}