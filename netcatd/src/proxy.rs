use std::{error, fmt, mem, thread};
use std::borrow::Cow;
use std::error::Error;
use std::fmt::Display;
use std::sync::Arc;
use std::sync::Mutex;
use termio::input::{Event, EventReader};
use util::shared::{Object, Shared};
use util::socket::{set_linger, set_reuse_port};
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio::net::TcpStream;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

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
    async fn encode(&self, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
        match self {
            Host::Dns(b) => {
                stream.write_all(&[HOST_DNS, b.len() as u8]).await?;
                stream.write_all(b).await?;
            }
            Host::V4(a) => {
                stream.write_u8(HOST_4).await?;
                stream.write_all(&a.octets()).await?;
            }
            Host::V6(a) => {
                stream.write_u8(HOST_6).await?;
                stream.write_all(&a.octets()).await?;
            }
        }
        Ok(())
    }
    async fn decode(stream: &mut TcpStream) -> Result<Self, Box<dyn Error>> {
        let addr_type = stream.read_u8().await?;
        match addr_type {
            HOST_4 => {
                let mut addr = [0u8; 4];
                stream.read_exact(&mut addr).await?;
                Ok(Host::V4(Ipv4Addr::from(addr)))
            }
            HOST_DNS => {
                let addr_len = stream.read_u8().await?;
                let mut addr = vec![0u8; addr_len as usize];
                stream.read_exact(&mut addr).await?;
                Ok(Host::Dns(addr))
            }
            HOST_6 => {
                let mut addr = [0u8; 16];
                stream.read_exact(&mut addr).await?;
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

pub async fn run_proxy_client(stream: &mut TcpStream, host: Host, port: u16) -> Result<(Host, u16), Box<dyn Error>> {
    stream.write_all(&[SOCKS5, 1, AUTH_NONE]).await?;
    let version = stream.read_u8().await?;
    if version != SOCKS5 {
        Err("Bad version")?;
    }
    let auth = stream.read_u8().await?;
    if auth != AUTH_NONE {
        Err("Bad auth")?;
    }
    stream.write_all(&[SOCKS5, CONNECT, RESERVED]).await?;
    host.encode(stream).await?;
    stream.write_u16(port).await?;
    if stream.read_u8().await? != SOCKS5 {
        Err("Bad version")?;
    }
    if stream.read_u8().await? != SUCCESS {
        Err("Connection failed")?;
    }
    if stream.read_u8().await? != RESERVED {
        Err("Reserved != 0")?;
    }
    let host = Host::decode(stream).await?;
    let port = stream.read_u16().await?;
    Ok((host, port))
}

pub async fn run_proxy_server(mut stream: &mut TcpStream) -> Result<(Host, u16), Box<dyn Error>> {
    let stream = &mut stream;
    let version = stream.read_u8().await?;
    match version {
        SOCKS4 => Err("Socks v4")?,
        SOCKS5 => {}
        HTTP_CONNECT => Err("HTTP CONNECT")?,
        HTTP_GET => Err("HTTP GET")?,
        _ => Err(format!("unknown protocol {}", version))?,
    }
    let mut found = false;
    for _ in 0..stream.read_u8().await? {
        if stream.read_u8().await? == AUTH_NONE {
            found = true;
        }
    }
    if !found {
        Err("Needs auth")?;
    }
    stream.write_all(&[SOCKS5, AUTH_NONE]).await?;
    if stream.read_u8().await? != SOCKS5 {
        Err("Changed version")?;
    }
    if stream.read_u8().await? != CONNECT {
        Err("Not a connect request")?;
    }
    if stream.read_u8().await? != RESERVED {
        Err("Unknown reserved")?;
    }
    let host = Host::decode(stream).await?;
    let port = stream.read_u16().await?;
    stream.write_all(&[SOCKS5, SUCCESS, RESERVED]).await?;
    Host::V4(Ipv4Addr::new(4, 8, 15, 16)).encode(stream).await?;
    stream.write_all(&[23, 42]).await?;
    Ok((host, port))
}