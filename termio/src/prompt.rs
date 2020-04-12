use std::{fmt, io};
use std::io::Write;

use crate::Direction;
use crate::input::{Event, Key, KeyEvent, Modifier};
use crate::output::{Column, CursorRestore, CursorSave, Delete, DeleteLine, Insert, MoveDirection, SafeWrite};

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
            write!(result.inner, "{}\r\n", i);
        }
        write!(result.inner, "{}{}", result.prompt.last().unwrap(), result.input);
        result
    }
    pub fn inner(&mut self)->&mut W{
        &mut self.inner
    }
    pub fn log(&mut self, content: &str) {
        write!(self.inner, "{}", CursorSave);
        write!(self.inner, "\r\n");
        write!(self.inner, "{}\r", MoveDirection(Direction::Up, self.prompt.len()));
        write!(self.inner, "{}{}", DeleteLine, content);
        for i in self.prompt.iter() {
            write!(self.inner, "\r\n{}{}", DeleteLine, i);
        }
        write!(self.inner, "{}", self.input);
        write!(self.inner, "{}", CursorRestore);
        write!(self.inner, "{}", MoveDirection(Direction::Down, 1));
    }
    pub fn update(&mut self, index: usize, content: &str) {
        write!(self.inner, "{}", CursorSave);
        write!(self.inner, "{}\r", MoveDirection(Direction::Up, self.prompt.len() - index - 1));
        write!(self.inner, "{}{}", DeleteLine, content);
        if index == self.prompt.len() - 1 {
            write!(self.inner, "{}", self.input);
        }
        self.prompt[index] = content.to_string();
        write!(self.inner, "{}", CursorRestore);
    }
    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor = 0;
        write!(self.inner, "{}{}{}", DeleteLine, Column(1), self.prompt.last().unwrap());
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
                write!(self.inner, "{}", MoveDirection(Direction::Left, 1));
            },
            Key::Arrow(Direction::Right) => if self.cursor < self.input.len() {
                self.cursor += 1;
                write!(self.inner, "{}", MoveDirection(Direction::Right, 1));
            },
            Key::Type('\r') => return false,
            Key::Type('\t') => return false,
            Key::Type(t) => {
                self.input.insert(self.cursor, t);
                self.cursor += 1;
                write!(self.inner, "{}{}", Insert(1), t);
            }
            Key::Delete => if self.cursor > 0 {
                self.input.remove(self.cursor - 1);
                self.cursor -= 1;
                write!(self.inner, "\x08{}", Delete(1));
            }
            Key::ForwardDelete => if self.cursor < self.input.len() {
                self.input.remove(self.cursor);
                write!(self.inner, "{}", Delete(1));
            }
            _ => return false
        }
        true
    }
    pub fn flush(&mut self){
        self.inner.flush();
    }
}