#![feature(never_type)]
#![allow(unused_imports)]
#![deny(unused_must_use)]

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate termio;

use std::any::Any;
use std::collections::HashMap;
use std::io::{ErrorKind, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use std::{error, io, mem, thread};

use termio::gui::event::BoxFnMut;
use termio::gui::gui::Gui;
use termio::input::{Event, EventReader};
use termio::screen::Screen;
use util::io::SafeWrite;
use util::rng::BoxRng;
use util::shared::{Object, Shared, SharedMut};
use util::socket::{set_linger, set_reuse_port};
use util::watch::Watchable;
use util::{watch, Name};

pub mod proxy;
pub mod tcp;
