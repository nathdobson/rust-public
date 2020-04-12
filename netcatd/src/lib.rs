use std::{io, mem, result, thread};
use std::fmt::Arguments;
use std::io::{ErrorKind, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use termio::input::{Event, EventReader};
use termio::output::SafeWrite;
use util::listen::{Listen, Listeners};
use util::object::Object;

use attempt::attempt;

pub mod demo;

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

pub struct NetcatPeer {
    stream: Option<Arc<TcpStream>>,
    buffer: Vec<u8>,
}

pub struct NetcatServer<H: NetcatHandler> {
    handler: Arc<Handler<H>>,
}


impl NetcatPeer {
    pub fn new(stream: Arc<TcpStream>) -> Self {
        NetcatPeer {
            stream: Some(stream),
            buffer: vec![],
        }
    }
    pub fn close(&mut self) {
        self.flush();
        if let Some(stream) = &mut self.stream {
            stream.shutdown(Shutdown::Both).ok();
        }
    }
}

impl SafeWrite for NetcatPeer {
    fn write_fmt(&mut self, args: Arguments) {
        SafeWrite::write_fmt(&mut self.buffer, args)
    }
    fn flush(&mut self) {
        if let Some(stream) = &mut self.stream {
            attempt!({
                catch!((&**stream).write_all(&self.buffer));
                catch!((&**stream).flush());
            }; catch (e : io::Error) => {
                println!("IO error during flush: {:?}", e);
                stream.shutdown(Shutdown::Both).ok();
                self.stream = None;
            });
        }
        self.buffer.clear();
    }
}


impl<H: NetcatHandler> Clone for NetcatServer<H> {
    fn clone(&self) -> Self {
        NetcatServer {
            handler: self.handler.clone(),
        }
    }
}

fn fix_listener(listener: &TcpListener) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        let optval: libc::c_int = 1;
        let ret = libc::setsockopt(listener.as_raw_fd(),
                                   libc::SOL_SOCKET,
                                   libc::SO_REUSEPORT,
                                   &optval as *const _ as *const libc::c_void,
                                   mem::size_of_val(&optval) as libc::socklen_t);
        if ret != 0 {
            let err: io::Result<()> = Err(io::Error::last_os_error());
            err.expect("setsockopt failed");
        }
    }
}

impl<H: NetcatHandler> NetcatServer<H> {
    pub fn new(handler: Arc<Handler<H>>) -> Self {
        NetcatServer {
            handler,
        }
    }
}

impl<H: NetcatHandler> NetcatServer<H> {
    fn handle_stream(&self, id: &Object, stream: Arc<TcpStream>) -> result::Result<(), PoisonError<()>> {
        let poison_listener = self.handler.lock()?.on_poison({
            let stream = stream.clone();
            move || { stream.shutdown(Shutdown::Both).ok(); }
        });
        self.handler.lock()?.add_peer(id, NetcatPeer::new(stream.clone()));
        let mut event_reader = EventReader::new(&*stream);
        loop {
            match event_reader.read() {
                Ok(event) => self.handler.lock()?.handle_event(id, &event),
                Err(error) => {
                    if error.kind() == ErrorKind::UnexpectedEof {
                        println!("Peer {:?} disconnected", id);
                    } else {
                        println!("Peer {:?} failed: {:?}", id, error);
                    }
                    stream.shutdown(Shutdown::Both).ok();
                    self.handler.lock()?.remove_peer(id);
                    break;
                }
            }
        }
        mem::drop(poison_listener);
        Ok(())
    }

    pub fn listen(&self, address: &str) -> io::Result<()> {
        let listener = Arc::new(TcpListener::bind(address)?);
        fix_listener(&listener);
        for (id, stream_result) in listener.incoming().enumerate() {
            let stream = stream_result?;
            let self2 = self.clone();
            thread::spawn(move || {
                self2.handle_stream(&Object::new(format!("peer_{}", id)), Arc::new(stream)).ok();
            });
        }
        Ok(())
    }
}

pub trait NetcatHandler: 'static + Send + Sized {
    fn add_peer(&mut self, id: &Object, peer: NetcatPeer);
    fn remove_peer(&mut self, id: &Object);
    fn handle_event(&mut self, id: &Object, event: &Event);
}
