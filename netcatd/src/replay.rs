use crate::{NetcatHandler, NetcatPeer};
use std::path::Path;
use termio::input::{Event, KeyEvent};
use std::collections::{HashSet, HashMap};
use std::fs::File;
use std::{fs, io};
use std::io::{BufReader, BufRead, Write};
use std::error::Error;

extern crate itertools;

use itertools::Itertools;

#[derive(Serialize, Deserialize)]
struct ReplayEvent {
    peer: String,
    event: Event,
}

pub struct ReplayHandler<H: NetcatHandler> {
    inner: H,
    input: Vec<ReplayEvent>,
    missing_peers: HashSet<String>,
    peers: HashMap<String, NetcatPeer>,
    output: File,
}

impl<H: NetcatHandler> ReplayHandler<H> {
    pub fn new(inner: H, directory: &Path) -> Result<Self, Box<dyn Error>> {
        let mut input_files: Vec<isize> =
            fs::read_dir(&directory)?
                .map_results(|e| e.file_name().to_str()?.parse::<isize>().ok())
                .filter_map(Result::transpose)
                .collect::<io::Result<Vec<_>>>()?;

        input_files.sort();
        let mut input = Vec::<ReplayEvent>::new();
        let peers = HashMap::new();
        let mut missing_peers = HashSet::<String>::new();
        for input_file in input_files.iter() {
            for line in BufReader::new(File::open(directory.join(input_file.to_string()))?).lines() {
                let line = line?;
                let event: ReplayEvent = serde_json::from_str(&line)?;
                missing_peers.insert(event.peer.clone());
                input.push(event);
            }
        }
        let output = input_files.iter().cloned().max().unwrap_or(-1) + 1;
        let output = File::create(directory.join(format!("{}", output)))?;
        Ok(ReplayHandler {
            inner,
            input,
            missing_peers,
            peers,
            output,
        })
    }
}

impl<H: NetcatHandler> NetcatHandler for ReplayHandler<H> {
    fn add_peer(&mut self, peer: &NetcatPeer) {
        self.inner.add_peer(peer);
        let id = peer.id().to_owned();
        self.missing_peers.remove(&id);
        self.peers.insert(id.clone(), peer.clone());
        println!("{:?} {:?} {:?}", id, self.missing_peers, self.peers);
        if self.missing_peers.is_empty() {
            for event in self.input.iter() {
                self.inner.handle_event(self.peers.get(&event.peer).unwrap(), &event.event);
            }
            self.peers.clear();
        }
    }

    fn remove_peer(&mut self, peer: &NetcatPeer) {
        self.inner.remove_peer(peer);
    }

    fn handle_event(&mut self, peer: &NetcatPeer, event: &Event) {
        if event != &Event::KeyEvent(KeyEvent::typed('q').control()) {
            writeln!(self.output, "{}", serde_json::to_string(&ReplayEvent { peer: peer.id.clone(), event: *event }).unwrap()).unwrap();
        }
        self.output.flush().unwrap();
        self.inner.handle_event(peer, event);
    }
}