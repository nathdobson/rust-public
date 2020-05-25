use std::path::Path;
use termio::input::{Event, KeyEvent};
use std::collections::{HashSet, HashMap, BTreeMap, BTreeSet};
use std::fs::File;
use std::{fs, io, mem, thread};
use std::io::{BufReader, BufRead, Write};
use std::error::Error;
use rand_xorshift::XorShiftRng;
use util::{completable, Name};

extern crate itertools;

use itertools::Itertools;
use std::sync::{Mutex, Arc, Weak};
use rand::{SeedableRng, RngCore};
use util::rng::BoxRng;
use crate::{Handler, Renderer, Timer, TimerCallback, timer};
use util::io::SafeWrite;
use termio::screen::Screen;
use std::any::Any;
use std::time::{Duration, Instant};
use util::any::AnyExt;
use std::ops::Add;

#[derive(Serialize, Deserialize, Debug)]
enum EventType {
    Add,
    Shutdown,
    Close,
    Input(Event),
    FireTimer,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReplayEvent {
    time: Duration,
    username: String,
    event: EventType,
}

pub struct ReplayHandlerBuilder {
    input: Vec<ReplayEvent>,
    output: File,
    fake_timer: Arc<ReplayTimer>,
    real_timer: Arc<dyn Timer>,
    timer_queue: timer::Receiver,
}

pub struct ReplayHandler {
    inner: Box<dyn Handler>,
    output: File,
    fake_timer: Arc<ReplayTimer>,
}

pub struct ReplayTimer {
    sender: timer::Sender,
    start: Instant,
    time: Mutex<Instant>,
    real_timer: Arc<dyn Timer>,
}

impl ReplayHandlerBuilder {
    pub fn new(directory: &Path, _: BoxRng, real_timer: Arc<dyn Timer>, _: Arc<dyn Renderer>) -> Result<Self, Box<dyn Error>> {
        let mut input_files: Vec<isize> =
            fs::read_dir(&directory)?
                .map_results(|e| e.file_name().to_str()?.parse::<isize>().ok())
                .filter_map(Result::transpose)
                .collect::<io::Result<Vec<_>>>()?;

        input_files.sort();
        let mut input = Vec::<ReplayEvent>::new();
        let mut max_time = Duration::default();
        for input_file in input_files.iter() {
            for line in BufReader::new(File::open(directory.join(input_file.to_string()))?).lines() {
                let line = line?;
                let event: ReplayEvent = serde_json::from_str(&line)?;
                max_time = max_time.max(event.time);
                input.push(event);
            }
        }
        let output = input_files.iter().cloned().max().unwrap_or(-1) + 1;
        let output = File::create(directory.join(format!("{}", output)))?;
        let (sender, timer_queue) = timer::channel();
        let start = Instant::now() - max_time;
        let fake_timer = Arc::new(ReplayTimer {
            sender,
            start,
            time: Mutex::new(start),
            real_timer: real_timer.clone(),
        });
        Ok(ReplayHandlerBuilder { input, output, fake_timer, timer_queue, real_timer })
    }
    pub fn rng(&self) -> BoxRng {
        Box::new(XorShiftRng::from_seed(*b"www.xkcd.com/221"))
    }
    pub fn timer(&self) -> Arc<dyn Timer> {
        self.fake_timer.clone()
    }
    pub fn build(self, mut inner: Box<dyn Handler>) -> ReplayHandler {
        let mut open = HashSet::<Name>::new();
        let mut half_open = HashSet::<Name>::new();
        for event in self.input.iter() {
            self.fake_timer.set(event.time);
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
                EventType::FireTimer => {
                    let next = self.timer_queue.inner.try_recv().unwrap().1;
                    next(&mut *inner)
                }
            }
        }
        thread::spawn({
            let real_timer = self.real_timer.clone();
            let timer_queue = self.timer_queue;
            move || {
                while let Ok((time, callback)) = timer_queue.inner.recv() {
                    real_timer.schedule(time, Box::new(move |handler| {
                        let this = handler.downcast_mut_result::<ReplayHandler>().unwrap();
                        let time = this.fake_timer.bump();
                        callback(&mut *this.inner);
                        this.write(ReplayEvent {
                            time,
                            username: "".to_string(),
                            event: EventType::FireTimer,
                        });
                    }))
                }
            }
        });
        let mut result = ReplayHandler {
            inner,
            output: self.output,
            fake_timer: self.fake_timer,
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

impl Timer for ReplayTimer {
    fn now(&self) -> Instant {
        *self.time.lock().unwrap()
    }

    fn schedule(&self, time: Instant, callback: Box<dyn FnOnce(&mut dyn Handler) + Send + 'static>) {
        self.sender.schedule(time, callback)
    }
}

impl ReplayTimer {
    fn bump(&self) -> Duration {
        let now = self.real_timer.now();
        *self.time.lock().unwrap() = now;
        now.duration_since(self.start)
    }
    fn set(&self, time: Duration) {
        *self.time.lock().unwrap() = self.start + time;
    }
}

impl Handler for ReplayHandler {
    fn peer_add(&mut self, username: &Name) {
        let time = self.fake_timer.bump();
        self.inner.peer_add(username);
        self.write(ReplayEvent {
            time,
            username: (**username).clone(),
            event: EventType::Add,
        });
    }

    fn peer_shutdown(&mut self, username: &Name) {
        let time = self.fake_timer.bump();
        self.inner.peer_shutdown(username);
        self.write(ReplayEvent {
            time,
            username: (**username).clone(),
            event: EventType::Shutdown,
        });
    }

    fn peer_close(&mut self, username: &Name) {
        let time = self.fake_timer.bump();
        self.inner.peer_close(username);
        self.write(ReplayEvent {
            time,
            username: (**username).clone(),
            event: EventType::Close,
        });
    }

    fn peer_event(&mut self, username: &Name, event: &Event) {
        let time = self.fake_timer.bump();
        self.inner.peer_event(username, event);
        if event != &Event::KeyEvent(KeyEvent::typed('q').control()) {
            self.write(ReplayEvent {
                time,
                username: (**username).clone(),
                event: EventType::Input(*event),
            });
        }
    }

    fn peer_render(&mut self, username: &Arc<String>, output: &mut Vec<u8>) {
        self.inner.peer_render(username, output);
    }
}