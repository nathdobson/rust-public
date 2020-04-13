use std::collections::HashMap;
use std::ops::{Index, IndexMut};
use std::mem;

pub struct Bag<T> {
    next: usize,
    map: HashMap<usize, T>,
}

impl<T> Bag<T> {
    pub fn new() -> Self {
        Bag {
            next: 0,
            map: HashMap::new(),
        }
    }
    pub fn push(&mut self, value: T) -> Token {
        let result = self.next;
        self.map.insert(result, value);
        self.next += 1;
        Token(result)
    }
    pub fn into_iter(self) -> impl Iterator<Item=(Token, T)> {
        self.map.into_iter().map(|(k, v)| (Token(k), v))
    }
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=(Token, &'a T)> {
        self.map.iter().map(|(k, v)| (Token(*k), v))
    }
    pub fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item=(Token, &'a mut T)> {
        self.map.iter_mut().map(|(k, v)| (Token(*k), v))
    }
    pub fn drain<'a>(&'a mut self) -> impl Iterator<Item=T> + 'a {
        self.map.drain().map(|(_, v)| v)
    }
    pub fn take(&mut self) -> impl Iterator<Item=T> {
        let map = mem::replace(&mut self.map, HashMap::new());
        map.into_iter().map(|(_, v)| v)
    }
    pub fn remove(&mut self, key: Token) -> T {
        self.map.remove(&key.0).unwrap()
    }
}

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub struct Token(usize);

impl<T> Index<Token> for Bag<T> {
    type Output = T;

    fn index(&self, index: Token) -> &T {
        self.map.get(&index.0).unwrap()
    }
}

impl<T> IndexMut<Token> for Bag<T> {
    fn index_mut(&mut self, index: Token) -> &mut Self::Output {
        self.map.get_mut(&index.0).unwrap()
    }
}