use std::{error, io, mem, thread};
use std::io::{ErrorKind, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::Mutex;

use termio::input::{Event, EventReader};
use termio::write::SafeWrite;
use util::shared::{Object, Shared};
use util::socket::{set_linger, set_reuse_port};

pub type NetcatPeer = Shared<NetcatPeerInner>;

#[derive(Debug)]
pub struct NetcatPeerInner {
    stream: TcpStream,
    id: String,
}

fn _is_object(peer: NetcatPeer) -> Object {
    peer.as_object()
}

pub struct NetcatServer {
    listener: Arc<TcpListener>,
    handler: Arc<Mutex<dyn NetcatHandler>>,
}

impl NetcatPeerInner {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        Ok(NetcatPeerInner {
            id: stream.peer_addr()?.to_string(),
            stream,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn close(&self) {
        self.stream.shutdown(Shutdown::Both).ok();
    }
}

impl Write for &NetcatPeerInner {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match (&self.stream).write(buf) {
            Ok(n) => return Ok(n),
            Err(err) => {
                eprintln!("Write error: {:?}", err);
                self.stream.shutdown(Shutdown::Both).ok();
            }
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Err(err) = (&self.stream).flush() {
            eprintln!("Flush error: {:?}", err);
            self.stream.shutdown(Shutdown::Both).ok();
        }
        Ok(())
    }
}

impl SafeWrite for &NetcatPeerInner {}

impl Clone for NetcatServer {
    fn clone(&self) -> Self {
        NetcatServer {
            listener: self.listener.clone(),
            handler: self.handler.clone(),
        }
    }
}

impl NetcatServer {
    pub fn new(handler: Arc<Mutex<dyn NetcatHandler>>, address: &str) -> io::Result<Self> {
        let listener = Arc::new(TcpListener::bind(address)?);
        set_reuse_port(&listener);
        Ok(NetcatServer {
            listener,
            handler,
        })
    }
}


impl NetcatServer {
    fn handle_stream(&self, stream: TcpStream) -> Result<(), Box<dyn error::Error>> {
        set_linger(&stream);
        let peer = Shared::new(NetcatPeerInner::new(stream)?);

        self.handler.lock().unwrap().add_peer(&peer);
        let mut event_reader = EventReader::new(&peer.stream);
        loop {
            match event_reader.read() {
                Ok(event) => self.handler.lock().unwrap().handle_event(&peer, &event),
                Err(error) => {
                    if error.kind() == ErrorKind::UnexpectedEof {
                        println!("Peer {:?} disconnected", peer);
                    } else {
                        println!("Peer {:?} failed: {:?}", peer, error);
                    }
                    peer.close();
                    self.handler.lock().unwrap().remove_peer(&peer);
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn listen(&self) -> io::Result<()> {
        for stream_result in self.listener.incoming() {
            let stream = stream_result?;
            let self2 = self.clone();
            thread::spawn(move || {
                println!("Receive error {:?}", self2.handle_stream(stream));
            });
        }
        Ok(())
    }
}

pub trait NetcatHandler: 'static + Send {
    fn add_peer(&mut self, peer: &NetcatPeer);
    fn remove_peer(&mut self, id: &NetcatPeer);
    fn handle_event(&mut self, id: &NetcatPeer, event: &Event);
}

