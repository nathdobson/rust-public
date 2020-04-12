use std::sync::{Arc, Mutex};
use crate::bag::{Bag, Token};

pub struct Listeners<T> {
    bag: Arc<Mutex<Bag<T>>>,
}

pub struct Listen<T> {
    bag: Arc<Mutex<Bag<T>>>,
    key: Token,
}

impl<T> Listeners<T> {
    pub fn new() -> Self {
        Listeners {
            bag: Arc::new(Mutex::new(Bag::new()))
        }
    }
    pub fn add(&self, value: T) -> Listen<T> {
        Listen {
            bag: self.bag.clone(),
            key: self.bag.lock().unwrap().push(value),
        }
    }
    pub fn take(&self) -> impl Iterator<Item=T> {
        self.bag.lock().unwrap().take()
    }
}

impl<T> Drop for Listen<T> {
    fn drop(&mut self) {
        self.bag.lock().unwrap().remove(self.key);
    }
}