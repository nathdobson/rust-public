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
use crate::{Peer, Handler, PeerTrait};
use util::io::SafeWrite;

#[derive(Serialize, Deserialize, Debug)]
enum EventType {
    Added,
    Input(Event),
}

#[derive(Serialize, Deserialize, Debug)]
struct ReplayEvent {
    peer: String,
    event: EventType,
}

struct ReplayPeer {
    inner: completable::Receiver<Peer>,
}

pub struct ReplayHandler {
    inner: Arc<Mutex<dyn Handler>>,
    holes: HashMap<Name, completable::Sender<Peer>>,
    output: Option<File>,
}

pub struct ReplayHandlerBuilder {
    input: Vec<ReplayEvent>,
    output: File,
}

impl ReplayHandlerBuilder {
    pub fn new(directory: &Path) -> Result<Self, Box<dyn Error>> {
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
        Ok(ReplayHandlerBuilder {
            input,
            output,
        })
    }
    pub fn build(self, inner: Arc<Mutex<dyn Handler>>) -> Arc<Mutex<ReplayHandler>> {
        let mut lock = inner.lock().unwrap();
        let mut holes = HashMap::new();
        for input in self.input {
            let name = Arc::new(input.peer);
            match input.event {
                EventType::Added => {
                    let (sender, receiver) = completable::channel();
                    lock.add_peer(&name, Box::new(ReplayPeer { inner: receiver }));
                    holes.insert(name, sender);
                }
                EventType::Input(input) => {
                    lock.handle_event(&name, &input);
                }
            }
        }
        mem::drop(lock);
        Arc::new(Mutex::new(ReplayHandler {
            inner,
            holes,
            output: Some(self.output),
        }))
    }
    pub fn make_rng(&self) -> BoxRng {
        Box::new(XorShiftRng::from_seed(*b"www.xkcd.com/221"))
    }
}

impl SafeWrite for ReplayPeer {}

impl Write for ReplayPeer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(self.inner.get_mut().map(|w| w.safe_write(buf)).unwrap_or(buf.len()))
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.get_mut().map(|w| w.safe_flush()).ok();
        Ok(())
    }
}

impl PeerTrait for ReplayPeer {
    fn close(&mut self) {
        self.inner = completable::failure();
    }
}

impl Handler for ReplayHandler {
    fn add_peer(&mut self, username: &Name, peer: Peer) {
        if let Some(hole) = self.holes.remove(username) {
            hole.send(peer);
        } else {
            if let Some(output) = self.output.as_mut() {
                writeln!(output, "{}", serde_json::to_string(&ReplayEvent {
                    peer: (**username).clone(),
                    event: EventType::Added,
                }).unwrap()).unwrap();
                output.flush().unwrap();
            }
            self.inner.lock().unwrap().add_peer(username, peer);
        }
    }

    fn remove_peer(&mut self, username: &Name) {
        self.output = None;
        self.inner.lock().unwrap().remove_peer(username);
    }

    fn handle_event(&mut self, username: &Name, event: &Event) {
        if let Some(output) = self.output.as_mut() {
            if event != &Event::KeyEvent(KeyEvent::typed('q').control()) {
                writeln!(output, "{}", serde_json::to_string(&ReplayEvent {
                    peer: (**username).clone(),
                    event: EventType::Input(*event),
                }).unwrap()).unwrap();
            }
            output.flush().unwrap();
        }
        self.inner.lock().unwrap().handle_event(username, event);
    }
}