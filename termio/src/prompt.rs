extern crate std;

use std::{fmt, io};

use crate::Direction;
use crate::input::{Event, Key, KeyEvent, Modifier};
use crate::output::*;
use util::io::SafeWrite;

pub struct Prompt<W: SafeWrite> {
    inner: W,
    prompt: Vec<String>,
    input: String,
    cursor: usize,
}

impl<W: SafeWrite> Prompt<W> {
    pub fn new(inner: W, prompt: Vec<String>) -> Prompt<W> {
        let mut result = Prompt {
            inner,
            cursor: 0,
            prompt,
            input: "".to_string(),
        };
        for i in result.prompt[..result.prompt.len() - 1].iter() {
            swrite!(result.inner, "{}\r\n", i);
        }
        swrite!(result.inner, "{}{}", result.prompt.last().unwrap(), result.input);
        result
    }
    pub fn inner(&mut self) -> &mut W {
        &mut self.inner
    }
    pub fn log(&mut self, content: &str) {
        swrite!(self.inner, "{}", CursorSave);
        swrite!(self.inner, "\r\n");
        swrite!(self.inner, "{}\r", MoveDirection(Direction::Up, self.prompt.len()));
        swrite!(self.inner, "{}{}", DeleteLineAll, content);
        for i in self.prompt.iter() {
            swrite!(self.inner, "\r\n{}{}", DeleteLineAll, i);
        }
        swrite!(self.inner, "{}", self.input);
        swrite!(self.inner, "{}", CursorRestore);
        swrite!(self.inner, "{}", MoveDirection(Direction::Down, 1));
    }
    pub fn update(&mut self, index: usize, content: &str) {
        swrite!(self.inner, "{}", CursorSave);
        swrite!(self.inner, "{}\r", MoveDirection(Direction::Up, self.prompt.len() - index - 1));
        swrite!(self.inner, "{}{}", DeleteLineAll, content);
        if index == self.prompt.len() - 1 {
            swrite!(self.inner, "{}", self.input);
        }
        self.prompt[index] = content.to_string();
        swrite!(self.inner, "{}", CursorRestore);
    }
    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor = 0;
        swrite!(self.inner, "{}{}{}", DeleteLineAll, Column(1), self.prompt.last().unwrap());
    }
    pub fn get(&mut self) -> &str {
        &self.input
    }
    pub fn input(&mut self, input: &Event) -> bool {
        let (modifier, key) = match *input {
            Event::KeyEvent(KeyEvent { modifier, key }) => (modifier, key),
            _ => return false,
        };
        if modifier != Modifier::default() {
            return false;
        }
        match key {
            Key::Arrow(Direction::Left) => if self.cursor > 0 {
                self.cursor -= 1;
                swrite!(self.inner, "{}", MoveDirection(Direction::Left, 1));
            },
            Key::Arrow(Direction::Right) => if self.cursor < self.input.len() {
                self.cursor += 1;
                swrite!(self.inner, "{}", MoveDirection(Direction::Right, 1));
            },
            Key::Type('\r') => return false,
            Key::Type('\t') => return false,
            Key::Type(t) => {
                self.input.insert(self.cursor, t);
                self.cursor += 1;
                swrite!(self.inner, "{}{}", Insert(1), t);
            }
            Key::Delete => if self.cursor > 0 {
                self.input.remove(self.cursor - 1);
                self.cursor -= 1;
                swrite!(self.inner, "\x08{}", Delete(1));
            }
            Key::ForwardDelete => if self.cursor < self.input.len() {
                self.input.remove(self.cursor);
                swrite!(self.inner, "{}", Delete(1));
            }
            _ => return false
        }
        true
    }
    pub fn flush(&mut self) {
        self.inner.safe_flush()
    }
}