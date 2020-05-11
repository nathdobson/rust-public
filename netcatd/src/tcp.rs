use std::{error, io, mem, thread};
use std::io::{ErrorKind, Write, Read};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::Mutex;

use termio::input::{Event, EventReader};
use util::shared::{Object, Shared};
use util::socket::{set_linger, set_reuse_port};
use util::io::SafeWrite;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use crate::{Handler, PeerTrait};


#[derive(Debug)]
struct PeerImpl {
    stream: Shared<TcpStream>,
}

pub struct NetcatServer {
    listener: Arc<TcpListener>,
    handler: Arc<Mutex<dyn Handler>>,
}

impl PeerImpl {
    pub fn new(stream: Shared<TcpStream>) -> io::Result<Self> {
        Ok(PeerImpl {
            stream,
        })
    }
}

impl Write for PeerImpl {
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

impl SafeWrite for PeerImpl {}

impl PeerTrait for PeerImpl {
    fn close(&mut self) {
        self.stream.shutdown(Shutdown::Both).ok();
    }
}

impl Clone for NetcatServer {
    fn clone(&self) -> Self {
        NetcatServer {
            listener: self.listener.clone(),
            handler: self.handler.clone(),
        }
    }
}

impl NetcatServer {
    pub fn new(handler: Arc<Mutex<dyn Handler>>, address: &str) -> io::Result<Self> {
        let listener = Arc::new(TcpListener::bind(address)?);
        //set_reuse_port(&listener);
        Ok(NetcatServer {
            listener,
            handler,
        })
    }
}

impl Drop for PeerImpl {
    fn drop(&mut self) {
        self.close();
    }
}

impl NetcatServer {
    fn handle_stream(&self, mut stream: Shared<TcpStream>) -> Result<(), Box<dyn error::Error>> {
        let version = stream.read_u8()?;
        let mut username = stream.peer_addr()?.to_string();
        if version == 4 {
            Err("Socks v4")?;
        } else if version == 5 {
            let auths = stream.read_u8()?;
            let mut found = false;
            for _ in 0..auths {
                if stream.read_u8()? == 0 {
                    found = true;
                }
            }
            if !found {
                Err("Needs auth")?;
            }
            stream.write_all(&[5, 0])?;
            if stream.read_u8()? != 5 {
                Err("Changed version")?;
            }
            if stream.read_u8()? != 1 {
                Err("Not a connect request")?;
            }
            if stream.read_u8()? != 0 {
                Err("Reserved != 0")?;
            }
            let addr_type = stream.read_u8()?;
            match addr_type {
                1 => {
                    let mut addr = [0u8; 4];
                    stream.read_exact(&mut addr)?;
                    username = Ipv4Addr::from(addr).to_string();
                }
                3 => {
                    let addr_len = stream.read_u8()?;
                    let mut addr = vec![0u8; addr_len as usize];
                    stream.read_exact(&mut addr)?;
                    username = String::from_utf8(addr)?;
                }
                4 => {
                    let mut addr = [0u8; 16];
                    stream.read_exact(&mut addr)?;
                    username = Ipv6Addr::from(addr).to_string();
                }
                _ => Err(format!("Unknown address type {}", addr_type))?
            }
            let mut _port = stream.read_u16::<BigEndian>()?;
            stream.write_all(&[5, 0, 0, 1, 4, 8, 15, 16, 23, 42])?;
        } else if version == b'C' {
            Err("HTTP proxy")?;
        } else if version == b'G' {
            Err("GET request")?;
        } else {
            Err(format!("Unknown c1 {} ", version))?;
        }
        //set_linger(&stream);
        let peer = Box::new(PeerImpl::new(stream.clone())?);
        let username = Arc::new(username);
        self.handler.lock().unwrap().add_peer(&username, peer);
        let mut event_reader = EventReader::new(stream.clone());
        loop {
            match event_reader.read() {
                Ok(event) => self.handler.lock().unwrap().handle_event(&username, &event),
                Err(error) => {
                    if error.kind() == ErrorKind::UnexpectedEof {
                        println!("Peer {:?} disconnected", username);
                    } else {
                        println!("Peer {:?} failed: {:?}", username, error);
                    }
                    stream.shutdown(Shutdown::Both).ok();
                    self.handler.lock().unwrap().remove_peer(&username);
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
                println!("Receive error {:?}", self2.handle_stream(Shared::new(stream)));
            });
        }
        Ok(())
    }
}
