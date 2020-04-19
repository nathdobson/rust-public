use core::str::utf8_char_width;
use std::{fmt, io, mem};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read};
use std::mem::MaybeUninit;
use std::ops::BitOr;

use byteorder::ReadBytesExt;
use itertools::Itertools;

use crate::Direction;
use crate::tokenizer::Tokenizer;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
pub enum Mouse {
    Up,
    ScrollDown,
    ScrollUp,
    Down(u8),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
pub enum Key {
    Arrow(Direction),
    Type(char),
    Func(u8),
    Delete,
    ForwardDelete,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Default)]
pub struct Modifier {
    pub shift: bool,
    pub control: bool,
    pub option: bool,
}

impl Modifier {
    pub const fn new() -> Modifier {
        Modifier {
            shift: false,
            control: false,
            option: false,
        }
    }
}

pub mod modifiers {
    use crate::input::Modifier;

    pub const PLAIN: Modifier = Modifier::new();
    pub const OPTION: Modifier = Modifier { option: true, ..Modifier::new() };
    pub const CONTROL: Modifier = Modifier { control: true, ..Modifier::new() };
    pub const SHIFT: Modifier = Modifier { shift: true, ..Modifier::new() };
}

impl BitOr<Modifier> for Modifier {
    type Output = Modifier;

    fn bitor(self, rhs: Modifier) -> Self::Output {
        Modifier {
            shift: self.shift || rhs.shift,
            control: self.control || rhs.control,
            option: self.option || rhs.option,
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
pub struct KeyEvent {
    pub modifier: Modifier,
    pub key: Key,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
pub struct MouseEvent {
    pub modifier: Modifier,
    pub mouse: Mouse,
    pub motion: bool,
    pub position: (isize, isize),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy)]
pub enum Event {
    KeyEvent(KeyEvent),
    MouseEvent(MouseEvent),
    Focus(bool),
    WindowPosition(isize, isize),
    WindowSize(isize, isize),
}

#[derive(Debug)]
pub struct Unknown;

#[derive(Debug)]
pub struct ParseError(Vec<u8>);

type EventResult = Result<Event, Unknown>;

impl fmt::Debug for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut set = f.debug_set();
        if self.shift {
            set.entry(&"shift");
        }
        if self.control {
            set.entry(&"control");
        }
        if self.option {
            set.entry(&"option");
        }
        set.finish()
    }
}

impl KeyEvent {
    pub fn typed(c: char) -> Self {
        KeyEvent { key: Key::Type(c), modifier: Modifier::default() }
    }
    pub fn control(mut self) -> Self {
        self.modifier.control = true;
        self
    }
    pub fn shift(mut self) -> Self {
        self.modifier.shift = true;
        self
    }
    pub fn option(mut self) -> Self {
        self.modifier.option = true;
        self
    }
}

pub struct EventReader<R: Read> {
    inner: Tokenizer<BufReader<R>>,
}

impl<R: Read> EventReader<R> {
    pub fn new(inner: R) -> EventReader<R> {
        EventReader {
            inner: Tokenizer::new(BufReader::new(inner)),
        }
    }
    pub fn read(&mut self) -> io::Result<Event> {
        loop {
            match self.read_maybe()? {
                Ok(event) => return Ok(event),
                Err(unknown) => println!("{:?}", unknown),
            }
        }
    }
    pub fn read_maybe(&mut self) -> io::Result<Result<Event, ParseError>> {
        self.inner.clear_log();
        Ok(match self.read_maybe_impl()? {
            Ok(event) => Ok(event),
            Err(Unknown) => Err(ParseError(self.inner.take_log())),
        })
    }
    fn read_maybe_impl(&mut self) -> io::Result<EventResult> {
        use crate::input::Event::Focus;
        use crate::input::Key::*;
        use crate::Direction::*;
        use crate::input::modifiers::*;
        Ok(match self.read_char()? {
            b1 @ '\t' | b1 @ '\r' | b1 @ ' '..='~' | b1 @ '\u{0080}'.. => self.known(PLAIN, Type(b1)),
            b1 @ '\x01'..='\x1a' => self.known(CONTROL, Type(char::from(((b1 as u8) - 1) + b'a'))),
            '\0' => self.known(CONTROL, Type(' ')),
            '\x1c' => self.known(CONTROL, Type('\\')),
            '\x1d' => self.known(CONTROL, Type(']')),
            '\x1e' => self.known(CONTROL, Type('^')),
            '\x1f' => self.known(CONTROL, Type('-')),
            '\x7f' => self.known(PLAIN, Delete),
            '\x1b' => match self.read_char()? {
                '[' => self.read_csi()?,
                '\x1b' => match self.read_char()? {
                    '[' => match self.read_char()? {
                        'A' => self.known(OPTION, Arrow(Up)),
                        'B' => self.known(OPTION, Arrow(Down)),
                        'C' => self.known(OPTION | SHIFT, Arrow(Right)),
                        'D' => self.known(OPTION | SHIFT, Arrow(Left)),
                        'Z' => self.known(OPTION | SHIFT, Type('\t')),
                        _ => Err(Unknown),
                    }
                    _ => Err(Unknown),
                }
                'O' => match self.read_char()? {
                    'P' => self.known(PLAIN, Func(1)),
                    'Q' => self.known(PLAIN, Func(2)),
                    'R' => self.known(PLAIN, Func(3)),
                    'S' => self.known(PLAIN, Func(4)),
                    _ => Err(Unknown),
                }
                'f' => self.known(OPTION, Arrow(Right)),
                'b' => self.known(OPTION, Arrow(Left)),
                '\x08' => self.known(OPTION | SHIFT, Delete),
                '(' => self.known(OPTION, ForwardDelete),
                b2 @ '\t' | b2 @ '\r' | b2 @ ' '..='\x7e' =>
                    self.known(OPTION, Type(b2)),
                b2 @ '\x01'..='\x1A' => self.known(CONTROL | OPTION, Type(char::from((b2 as u8 - 1) + b'a'))),
                '\x00' => self.known(CONTROL | OPTION, Type(' ')),
                '\x1c' => self.known(CONTROL | OPTION, Type('\\')),
                '\x1d' => self.known(CONTROL | OPTION, Type(']')),
                '\x1e' => self.known(CONTROL | OPTION, Type('^')),
                '\x1f' => self.known(CONTROL | OPTION, Type('-')),
                '\x7f' => self.known(OPTION, Delete),
                _ => Err(Unknown)
            }
        })
    }
    fn read_csi(&mut self) -> io::Result<EventResult> {
        use crate::input::Event::*;
        use crate::input::Key::*;
        use crate::Direction::*;
        use crate::input::modifiers::*;
        let params = self.read_numbers()?;
        Ok(match (params.as_slice(), self.read_char()?) {
            (&[], 'A') => self.known(PLAIN, Arrow(Up)),
            (&[], 'B') => self.known(PLAIN, Arrow(Down)),
            (&[1, 5], 'C') => self.known(CONTROL, Arrow(Right)),
            (&[1, 2], 'C') => self.known(SHIFT, Arrow(Right)),
            (&[], 'C') => self.known(PLAIN, Arrow(Right)),
            (&[1, 5], 'D') => self.known(CONTROL, Arrow(Left)),
            (&[1, 2], 'D') => self.known(SHIFT, Arrow(Left)),
            (&[3], '~') => self.known(PLAIN, ForwardDelete),
            (&[3, 5], '~') => self.known(CONTROL, ForwardDelete),
            (&[3, 2], '~') => self.known(SHIFT, ForwardDelete),
            (&[], 'D') => self.known(PLAIN, Arrow(Left)),
            (&[15], '~') => self.known(PLAIN, Func(5)),
            (&[17], '~') => self.known(PLAIN, Func(6)),
            (&[18], '~') => self.known(PLAIN, Func(7)),
            (&[19], '~') => self.known(PLAIN, Func(8)),
            (&[20], '~') => self.known(PLAIN, Func(9)),
            (&[21], '~') => self.known(PLAIN, Func(10)),
            (&[23], '~') => self.known(PLAIN, Func(11)),
            (&[24], '~') => self.known(PLAIN, Func(12)),
            (&[], 'M') => self.read_button()?,
            (&[], 'I') => Ok(Focus(true)),
            (&[], 'O') => Ok(Focus(false)),
            (&[], 'Z') => self.known(SHIFT, Type('\t')),
            (&[3, x, y], 't') => Ok(WindowPosition(x, y)),
            (&[4, w, h], 't') => Ok(WindowSize(w, h)),
            _ => Err(Unknown),
        })
    }
    fn read_button(&mut self) -> io::Result<Result<Event, Unknown>> {
        let (mut flags, mut x, mut y) = (self.read_u8()?, self.read_u8()?, self.read_u8()?);
        if flags < 32 || x < 32 || y < 32 {
            return Ok(Err(Unknown));
        }
        flags -= 32;
        x -= 32;
        y -= 32;
        let mut modifier = Modifier::default();
        let mut motion = false;
        let mut button = 0;
        for i in 0..8 {
            match flags & (1 << i) {
                1 => button |= 1,
                2 => button |= 2,
                4 => modifier.shift = true,
                8 => modifier.option = true,
                16 => modifier.control = true,
                32 => motion = true,
                64 => button |= 4,
                128 => button |= 8,
                _ => {}
            }
        }
        let mouse = match button {
            3 => Mouse::Up,
            4 => Mouse::ScrollUp,
            5 => Mouse::ScrollDown,
            n => Mouse::Down(n),
        };
        let position = (x as isize, y as isize);
        Ok(Ok(Event::MouseEvent(MouseEvent { modifier, mouse, motion, position })))
    }
    fn read_char(&mut self) -> io::Result<char> {
        let b1 = self.read_u8()?;
        let mut buf = [b1, 0, 0, 0];
        let w = utf8_char_width(b1);
        if w == 0 {
            return Ok(b1 as char);
        }
        let slice = &mut buf[1..w];
        self.inner.read_exact(slice)?;
        let s = std::str::from_utf8(&mut buf[0..w]).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
        Ok(s.chars().exactly_one().unwrap())
    }
    fn peek(&mut self) -> io::Result<Option<u8>> {
        let buf = self.inner.fill_buf()?;
        if buf.len() == 0 {
            return Ok(None);
        }
        Ok(Some(buf[0]))
    }
    fn read_while(&mut self, mut pred: impl FnMut(u8) -> bool) -> io::Result<Vec<u8>> {
        let mut result = vec![];
        while let Some(c) = self.peek()? {
            if !pred(c) {
                break;
            }
            result.push(c);
            self.inner.consume(1);
        }
        Ok(result)
    }
    fn read_number(&mut self) -> io::Result<Option<isize>> {
        let vec = self.read_while(|c| c.is_ascii_digit())?;
        if vec.len() == 0 {
            return Ok(None);
        } else {
            return Ok(Some(
                std::str::from_utf8(&vec)
                    .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?.parse()
                    .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?));
        }
    }
    fn read_numbers(&mut self) -> io::Result<Vec<isize>> {
        let mut result = vec![];
        while let Some(n) = self.read_number()? {
            result.push(n);
            if self.peek()? == Some(b';') {
                self.inner.consume(1);
                continue;
            } else {
                break;
            }
        }
        Ok(result)
    }
    fn read_u8(&mut self) -> io::Result<u8> {
        self.inner.read_u8()
    }
    fn known(&mut self, modifier: Modifier, key: Key) -> EventResult {
        Ok(Event::KeyEvent(KeyEvent { modifier, key }))
    }
}