#![feature(never_type)]
#![allow(unused_imports)]

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
use util::listen::{Listen, Listeners};
use util::socket::{set_reuse_port, set_linger};
use util::shared::{Shared, SharedMut};
use util::shared::Object;
use util::io::SafeWrite;
use util::{Name, watch};
use std::time::{Duration, Instant};
use termio::screen::Screen;
use std::collections::HashMap;
use util::watch::Watchable;
use util::rng::BoxRng;
use std::any::Any;
use util::any::Upcast;

pub mod replay;
pub mod tcp;
pub mod timer;
pub mod proxy;

pub trait Renderer: Send + Sync + 'static {
    fn peer_render(&self, username: &Name);
    fn peer_shutdown(&self, username: &Name);
}

type TimerCallback = Box<dyn FnOnce(&mut dyn Handler) + Send + 'static>;

pub trait Timer: Send + Sync + 'static {
    fn now(&self) -> Instant;
    fn schedule(&self, time: Instant, callback: TimerCallback);
}

pub trait Handler: 'static + Send + Upcast<dyn Any> {
    fn peer_add(&mut self, username: &Name);
    fn peer_shutdown(&mut self, username: &Name);
    fn peer_close(&mut self, username: &Name);
    fn peer_event(&mut self, username: &Name, event: &Event);
    fn peer_render(&mut self, username: &Name, output: &mut Vec<u8>);
}
