use std::sync::{mpsc, Mutex, Arc, Condvar};
use std::sync::atomic::AtomicBool;
use crate::gui::gui::Gui;
use std::thread::JoinHandle;
use std::thread;
use util::lossy;

