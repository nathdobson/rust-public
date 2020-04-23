extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate termio;

use std::{error, io, mem, thread};
use std::io::{ErrorKind, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use termio::input::{Event, EventReader};
use termio::write::SafeWrite;
use util::listen::{Listen, Listeners};
use util::socket::{set_reuse_port, set_linger};
use util::shared::Shared;
use util::shared::Object;

pub mod demo;
pub mod replay;

pub struct Handler<T> {
    inner: Mutex<T>,
    poison_listeners: Listeners<Box<dyn FnOnce() + Send>>,
}

struct HandlerGuard<'a, T> {
    inner: &'a Handler<T>,
    guard: MutexGuard<'a, T>,
}

impl<T> Handler<T> {
    pub fn new(inner: T) -> Self {
        Handler {
            inner: Mutex::new(inner),
            poison_listeners: Listeners::new(),
        }
    }
    fn lock<'a>(&'a self) -> Result<HandlerGuard<'a, T>, PoisonError<()>> {
        Ok(HandlerGuard {
            inner: self,
            guard: self.inner.lock().map_err(|_| PoisonError::new(()))?,
        })
    }
}

impl<'a, T> HandlerGuard<'a, T> {
    fn on_poison(&self, callback: impl FnOnce() + 'static + Send) -> Listen<Box<dyn FnOnce() + Send>> {
        self.inner.poison_listeners.add(Box::new(callback))
    }
}

impl<'a, T> Deref for HandlerGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a, T> DerefMut for HandlerGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

impl<'a, T> Drop for HandlerGuard<'a, T> {
    fn drop(&mut self) {
        if thread::panicking() {
            let listeners = self.inner.poison_listeners.take();
            thread::spawn(|| {
                for listener in listeners {
                    listener()
                }
            });
        }
    }
}

pub type NetcatPeer = Shared<NetcatPeerInner>;

#[derive(Debug)]
pub struct NetcatPeerInner {
    stream: TcpStream,
    id: String,
}

fn _is_object(peer: NetcatPeer) -> Object {
    peer.as_object()
}

pub struct NetcatServer<H: NetcatHandler> {
    listener: Arc<TcpListener>,
    handler: Arc<Handler<H>>,
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

impl<H: NetcatHandler> Clone for NetcatServer<H> {
    fn clone(&self) -> Self {
        NetcatServer {
            listener: self.listener.clone(),
            handler: self.handler.clone(),
        }
    }
}

impl<H: NetcatHandler> NetcatServer<H> {
    pub fn new(handler: Arc<Handler<H>>, address: &str) -> io::Result<Self> {
        let listener = Arc::new(TcpListener::bind(address)?);
        set_reuse_port(&listener);
        //set_reuse_addr(&listener);
        Ok(NetcatServer {
            listener,
            handler,
        })
    }
}


impl<H: NetcatHandler> NetcatServer<H> {
    fn handle_stream(&self, stream: TcpStream) -> Result<(), Box<dyn error::Error>> {
        set_linger(&stream);
        let peer = Shared::new(NetcatPeerInner::new(stream)?);
        let poison_listener = self.handler.lock()?.on_poison({
            let peer = peer.clone();
            move || { peer.close(); }
        });
        self.handler.lock()?.add_peer(&peer);
        let mut event_reader = EventReader::new(&peer.stream);
        loop {
            match event_reader.read() {
                Ok(event) => self.handler.lock()?.handle_event(&peer, &event),
                Err(error) => {
                    if error.kind() == ErrorKind::UnexpectedEof {
                        println!("Peer {:?} disconnected", peer);
                    } else {
                        println!("Peer {:?} failed: {:?}", peer, error);
                    }
                    peer.close();
                    self.handler.lock()?.remove_peer(&peer);
                    break;
                }
            }
        }
        mem::drop(poison_listener);
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

pub trait NetcatHandler: 'static + Send + Sized {
    fn add_peer(&mut self, peer: &NetcatPeer);
    fn remove_peer(&mut self, id: &NetcatPeer);
    fn handle_event(&mut self, id: &NetcatPeer, event: &Event);
}

