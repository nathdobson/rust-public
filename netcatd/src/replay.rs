use std::path::Path;
use termio::input::{Event, KeyEvent};
use std::collections::{HashSet, HashMap, BTreeMap, BTreeSet};
use std::fs::File;
use std::{fs, io};
use std::io::{BufReader, BufRead, Write};
use std::error::Error;
use rand_xorshift::XorShiftRng;

extern crate itertools;

use itertools::Itertools;
use crate::tcp::{NetcatHandler, NetcatPeer};
use std::sync::{Mutex, Arc};
use rand::{SeedableRng, RngCore};
use util::rng::BoxRng;

#[derive(Serialize, Deserialize, Debug)]
struct ReplayEvent {
    peer: String,
    event: Event,
}

pub struct ReplayHandler {
    inner: Arc<Mutex<dyn NetcatHandler>>,
    missing_peers: HashSet<String>,
    peers: BTreeMap<String, NetcatPeer>,
    input: Vec<ReplayEvent>,
    output: File,
}

pub struct ReplayHandlerBuilder {
    input: Vec<ReplayEvent>,
    peers: HashSet<String>,
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
        let mut peers = HashSet::new();
        for input_file in input_files.iter() {
            for line in BufReader::new(File::open(directory.join(input_file.to_string()))?).lines() {
                let line = line?;
                let event: ReplayEvent = serde_json::from_str(&line)?;
                peers.insert(event.peer.clone());
                input.push(event);
            }
        }
        let output = input_files.iter().cloned().max().unwrap_or(-1) + 1;
        let output = File::create(directory.join(format!("{}", output)))?;
        Ok(ReplayHandlerBuilder {
            input,
            peers,
            output,
        })
    }
    pub fn build(self, inner: Arc<Mutex<dyn NetcatHandler>>) -> Arc<Mutex<ReplayHandler>> {
        Arc::new(Mutex::new(ReplayHandler {
            inner,
            missing_peers: self.peers,
            peers: BTreeMap::new(),
            input: self.input,
            output: self.output,
        }))
    }
    pub fn make_rng(&self) -> BoxRng {
        Box::new(XorShiftRng::from_seed(*b"www.xkcd.com/221"))
    }
}

impl NetcatHandler for ReplayHandler {
    fn add_peer(&mut self, peer: &NetcatPeer) {
        let id = peer.id().to_owned();
        self.missing_peers.remove(&id);
        self.peers.insert(id.clone(), peer.clone());
        if self.missing_peers.is_empty() {
            let mut lock = self.inner.lock().unwrap();
            for (_, peer) in self.peers.iter() {
                lock.add_peer(peer);
            }
            for event in self.input.iter() {
                println!("{:?}", event);
                lock.handle_event(self.peers.get(&event.peer).unwrap(), &event.event);
            }
            self.peers.clear();
            self.input.clear();
        }
    }

    fn remove_peer(&mut self, peer: &NetcatPeer) {
        self.inner.lock().unwrap().remove_peer(peer);
    }

    fn handle_event(&mut self, peer: &NetcatPeer, event: &Event) {
        if self.missing_peers.is_empty() {
            if event != &Event::KeyEvent(KeyEvent::typed('q').control()) {
                writeln!(self.output, "{}", serde_json::to_string(&ReplayEvent { peer: peer.id().to_owned(), event: *event }).unwrap()).unwrap();
            }
            self.output.flush().unwrap();
            self.inner.lock().unwrap().handle_event(peer, event);
        }
    }
}