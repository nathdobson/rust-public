use std::{error, io, mem, thread, fmt};
use std::io::{ErrorKind, Write, Read};
use std::net::{Shutdown, TcpListener, TcpStream, IpAddr};
use std::sync::Arc;
use std::sync::Mutex;

use termio::input::{Event, EventReader};
use util::shared::{Object, Shared};
use util::socket::{set_linger, set_reuse_port};
use util::io::SafeWrite;
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian, LittleEndian};
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use crate::Handler;
use std::error::Error;
use std::borrow::Cow;
use std::fmt::Display;

const SOCKS5: u8 = 5;
const SOCKS4: u8 = 4;
const HTTP_GET: u8 = b'G';
const HTTP_CONNECT: u8 = b'C';

const CONNECT: u8 = 1;
const RESERVED: u8 = 0;
const SUCCESS: u8 = 0;
const AUTH_NONE: u8 = 0;
const HOST_4: u8 = 1;
const HOST_DNS: u8 = 3;
const HOST_6: u8 = 4;

#[derive(Clone)]
pub enum Host {
    Dns(Vec<u8>),
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

impl Host {
    fn encode(&self, mut stream: &TcpStream) -> Result<(), Box<dyn Error>> {
        match self {
            Host::Dns(b) => {
                stream.write_all(&[HOST_DNS, b.len() as u8])?;
                stream.write_all(b)?;
            }
            Host::V4(a) => {
                stream.write_u8(HOST_4)?;
                stream.write_all(&a.octets())?;
            }
            Host::V6(a) => {
                stream.write_u8(HOST_6)?;
                stream.write_all(&a.octets())?;
            }
        }
        Ok(())
    }
    fn decode(mut stream: &TcpStream) -> Result<Self, Box<dyn Error>> {
        let addr_type = stream.read_u8()?;
        match addr_type {
            HOST_4 => {
                let mut addr = [0u8; 4];
                stream.read_exact(&mut addr)?;
                Ok(Host::V4(Ipv4Addr::from(addr)))
            }
            HOST_DNS => {
                let addr_len = stream.read_u8()?;
                let mut addr = vec![0u8; addr_len as usize];
                stream.read_exact(&mut addr)?;
                Ok(Host::Dns(addr))
            }
            HOST_6 => {
                let mut addr = [0u8; 16];
                stream.read_exact(&mut addr)?;
                Ok(Host::V6(Ipv6Addr::from(addr)))
            }
            _ => Err(format!("Unknown address type {}", addr_type))?
        }
    }
    pub fn to_string(self) -> Result<String, Box<dyn Error>> {
        Ok(match self {
            Host::Dns(x) => String::from_utf8(x)?,
            Host::V4(x) => x.to_string(),
            Host::V6(x) => x.to_string(),
        })
    }
}

pub fn run_proxy_client(mut stream: &TcpStream, host: Host, port: u16) -> Result<(Host, u16), Box<dyn Error>> {
    stream.write_all(&[SOCKS5, 1, AUTH_NONE])?;
    let version = stream.read_u8()?;
    if version != SOCKS5 {
        Err("Bad version")?;
    }
    let auth = stream.read_u8()?;
    if auth != AUTH_NONE {
        Err("Bad auth")?;
    }
    stream.write_all(&[SOCKS5, CONNECT, RESERVED])?;
    host.encode(stream)?;
    stream.write_u16::<LittleEndian>(port)?;
    if stream.read_u8()? != SOCKS5 {
        Err("Bad version")?;
    }
    if stream.read_u8()? != SUCCESS {
        Err("Connection failed")?;
    }
    if stream.read_u8()? != RESERVED {
        Err("Reserved != 0")?;
    }
    let host = Host::decode(stream)?;
    let port = stream.read_u16::<BigEndian>()?;
    Ok((host, port))
}

pub fn run_proxy_server(mut stream: &TcpStream) -> Result<(Host, u16), Box<dyn Error>> {
    let stream = &mut stream;
    let version = stream.read_u8()?;
    match version {
        SOCKS4 => Err("Socks v4")?,
        SOCKS5 => {}
        HTTP_CONNECT => Err("HTTP CONNECT")?,
        HTTP_GET => Err("HTTP GET")?,
        _ => Err(format!("unknown protocol {}", version))?,
    }
    let mut found = false;
    for _ in 0..stream.read_u8()? {
        if stream.read_u8()? == AUTH_NONE {
            found = true;
        }
    }
    if !found {
        Err("Needs auth")?;
    }
    stream.write_all(&[SOCKS5, AUTH_NONE])?;
    if stream.read_u8()? != SOCKS5 {
        Err("Changed version")?;
    }
    if stream.read_u8()? != CONNECT {
        Err("Not a connect request")?;
    }
    if stream.read_u8()? != RESERVED {
        Err("Unknown reserved")?;
    }
    let host = Host::decode(stream)?;
    let port = stream.read_u16::<BigEndian>()?;
    stream.write_all(&[SOCKS5, SUCCESS, RESERVED])?;
    Host::V4(Ipv4Addr::new(4, 8, 15, 16)).encode(stream)?;
    stream.write_all(&[23, 42])?;
    Ok((host, port))
}