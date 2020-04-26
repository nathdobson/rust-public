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
use termio::write::SafeWrite;
use util::listen::{Listen, Listeners};
use util::socket::{set_reuse_port, set_linger};
use util::shared::Shared;
use util::shared::Object;

pub mod demo;
pub mod replay;
pub mod tcp;
//
//pub struct Handler<T: ?Sized> {
//    poison_listeners: Listeners<Box<dyn FnOnce() + Send>>,
//    inner: Mutex<T>,
//}
//
//pub struct HandlerGuard<'a, T: ?Sized> {
//    inner: &'a Handler<T>,
//    guard: MutexGuard<'a, T>,
//}
//
//impl<T: ?Sized> Handler<T> {
//    pub fn new(inner: T) -> Self where T: Sized {
//        Handler {
//            inner: Mutex::new(inner),
//            poison_listeners: Listeners::new(),
//        }
//    }
//    pub fn lock<'a>(&'a self) -> HandlerGuard<'a, T> {
//        HandlerGuard {
//            inner: self,
//            guard: self.inner.lock().map_err(|_| PoisonError::new(())).unwrap(),
//        }
//    }
//}
//
//impl<'a, T: ?Sized> HandlerGuard<'a, T> {
//    fn on_poison(&self, callback: impl FnOnce() + 'static + Send) -> Listen<Box<dyn FnOnce() + Send>> {
//        self.inner.poison_listeners.add(Box::new(callback))
//    }
//}
//
//impl<'a, T:?Sized> Deref for HandlerGuard<'a, T> {
//    type Target = T;
//
//    fn deref(&self) -> &Self::Target {
//        self.guard.deref()
//    }
//}
//
//impl<'a, T:?Sized> DerefMut for HandlerGuard<'a, T> {
//    fn deref_mut(&mut self) -> &mut Self::Target {
//        self.guard.deref_mut()
//    }
//}
//
//impl<'a, T: ?Sized> Drop for HandlerGuard<'a, T> {
//    fn drop(&mut self) {
//        if thread::panicking() {
//            let listeners = self.inner.poison_listeners.take();
//            thread::spawn(|| {
//                for listener in listeners {
//                    listener()
//                }
//            });
//        }
//    }
//}