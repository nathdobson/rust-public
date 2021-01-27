use std::{fmt, io, mem};
use std::collections::BTreeSet;
use std::io::{Error, ErrorKind};
use std::mem::MaybeUninit;
use std::ops::BitOr;
use async_std::io::Read;
use async_std::io::BufReader;

use byteorder::ReadBytesExt;
use itertools::Itertools;
use futures::io::AsyncRead;
use futures::io::AsyncReadExt;

use crate::Direction;
use crate::tokenizer::Tokenizer;
use std::time::Instant;
use std::pin::Pin;
use pin_project::pin_project;
use futures::{AsyncBufRead, AsyncBufReadExt};

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Hash)]
pub enum Mouse {
    Up,
    ScrollDown,
    ScrollUp,
    Down(u8),
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Hash)]
pub enum Key {
    Arrow(Direction),
    Type(char),
    Func(u8),
    Delete,
    ForwardDelete,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Default, Serialize, Deserialize, Hash)]
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

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Hash)]
pub struct KeyEvent {
    pub modifier: Modifier,
    pub key: Key,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize, Hash)]
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
    TextAreaSize(isize, isize),
    ScreenSize(isize, isize),
    CursorPosition(isize, isize),
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

#[pin_project]
pub struct EventReader<R: Read> {
    #[pin]
    inner: Tokenizer<BufReader<R>>,
}

fn known(modifier: Modifier, key: Key) -> EventResult {
    Ok(Event::KeyEvent(KeyEvent { modifier, key }))
}

impl<R: Read> EventReader<R> {
    pub fn new(inner: R) -> EventReader<R> {
        EventReader {
            inner: Tokenizer::new(BufReader::new(inner)),
        }
    }
    pub async fn read(mut self: Pin<&mut Self>) -> io::Result<Event> {
        loop {
            match self.as_mut().read_maybe().await? {
                Ok(event) => return Ok(event),
                Err(unknown) => eprintln!("{:?}", unknown),
            }
        }
    }
    pub async fn read_maybe(mut self: Pin<&mut Self>) -> io::Result<Result<Event, ParseError>> {
        self.as_mut().project().inner.clear_log();
        Ok(match self.as_mut().read_maybe_impl().await? {
            Ok(event) => Ok(event),
            Err(Unknown) => Err(ParseError(self.as_mut().project().inner.take_log())),
        })
    }
    async fn read_maybe_impl(mut self: Pin<&mut Self>) -> io::Result<EventResult> {
        use crate::input::Event::Focus;
        use crate::input::Key::*;
        use crate::Direction::*;
        use crate::input::modifiers::*;
        Ok(match self.as_mut().read_char().await? {
            b1 @ '\t' | b1 @ '\r' | b1 @ ' '..='~' | b1 @ '\u{0080}'.. => known(PLAIN, Type(b1)),
            b1 @ '\x01'..='\x1a' => known(CONTROL, Type(char::from(((b1 as u8) - 1) + b'a'))),
            '\0' => known(CONTROL, Type(' ')),
            '\x1c' => known(CONTROL, Type('\\')),
            '\x1d' => known(CONTROL, Type(']')),
            '\x1e' => known(CONTROL, Type('^')),
            '\x1f' => known(CONTROL, Type('-')),
            '\x7f' => known(PLAIN, Delete),
            '\x1b' => match self.as_mut().read_char().await? {
                '[' => self.as_mut().read_csi().await?,
                '\x1b' => match self.as_mut().read_char().await? {
                    '[' => match self.as_mut().read_char().await? {
                        'A' => known(OPTION, Arrow(Up)),
                        'B' => known(OPTION, Arrow(Down)),
                        'C' => known(OPTION | SHIFT, Arrow(Right)),
                        'D' => known(OPTION | SHIFT, Arrow(Left)),
                        'Z' => known(OPTION | SHIFT, Type('\t')),
                        _ => Err(Unknown),
                    }
                    _ => Err(Unknown),
                }
                'O' => match self.as_mut().read_char().await? {
                    'P' => known(PLAIN, Func(1)),
                    'Q' => known(PLAIN, Func(2)),
                    'R' => known(PLAIN, Func(3)),
                    'S' => known(PLAIN, Func(4)),
                    _ => Err(Unknown),
                }
                'f' => known(OPTION, Arrow(Right)),
                'b' => known(OPTION, Arrow(Left)),
                '\x08' => known(OPTION | SHIFT, Delete),
                '(' => known(OPTION, ForwardDelete),
                b2 @ '\t' | b2 @ '\r' | b2 @ ' '..='\x7e' =>
                    known(OPTION, Type(b2)),
                b2 @ '\x01'..='\x1A' => known(CONTROL | OPTION, Type(char::from((b2 as u8 - 1) + b'a'))),
                '\x00' => known(CONTROL | OPTION, Type(' ')),
                '\x1c' => known(CONTROL | OPTION, Type('\\')),
                '\x1d' => known(CONTROL | OPTION, Type(']')),
                '\x1e' => known(CONTROL | OPTION, Type('^')),
                '\x1f' => known(CONTROL | OPTION, Type('-')),
                '\x7f' => known(OPTION, Delete),
                _ => Err(Unknown)
            }
        })
    }
    async fn read_csi(mut self: Pin<&mut Self>) -> io::Result<EventResult> {
        use crate::input::Event::*;
        use crate::input::Key::*;
        use crate::Direction::*;
        use crate::input::modifiers::*;
        let params = self.as_mut().read_numbers().await?;
        Ok(match (params.as_slice(), self.as_mut().read_char().await?) {
            (&[], 'A') => known(PLAIN, Arrow(Up)),
            (&[], 'B') => known(PLAIN, Arrow(Down)),
            (&[1, 5], 'C') => known(CONTROL, Arrow(Right)),
            (&[1, 2], 'C') => known(SHIFT, Arrow(Right)),
            (&[], 'C') => known(PLAIN, Arrow(Right)),
            (&[1, 5], 'D') => known(CONTROL, Arrow(Left)),
            (&[1, 2], 'D') => known(SHIFT, Arrow(Left)),
            (&[3], '~') => known(PLAIN, ForwardDelete),
            (&[3, 5], '~') => known(CONTROL, ForwardDelete),
            (&[3, 2], '~') => known(SHIFT, ForwardDelete),
            (&[], 'D') => known(PLAIN, Arrow(Left)),
            (&[15], '~') => known(PLAIN, Func(5)),
            (&[17], '~') => known(PLAIN, Func(6)),
            (&[18], '~') => known(PLAIN, Func(7)),
            (&[19], '~') => known(PLAIN, Func(8)),
            (&[20], '~') => known(PLAIN, Func(9)),
            (&[21], '~') => known(PLAIN, Func(10)),
            (&[23], '~') => known(PLAIN, Func(11)),
            (&[24], '~') => known(PLAIN, Func(12)),
            (&[], 'M') => self.as_mut().read_button().await?,
            (&[], 'I') => Ok(Focus(true)),
            (&[], 'O') => Ok(Focus(false)),
            (&[], 'Z') => known(SHIFT, Type('\t')),
            (&[y, x], 'R') => Ok(CursorPosition(x, y)),
            (&[3, x, y], 't') => Ok(WindowPosition(x, y)),
            (&[4, h, w], 't') => Ok(WindowSize(w, h)),
            (&[8, h, w], 't') => Ok(TextAreaSize(w, h)),
            (&[9, h, w], 't') => Ok(ScreenSize(w, h)),
            _ => Err(Unknown),
        })
    }
    async fn read_button(mut self: Pin<&mut Self>) -> io::Result<Result<Event, Unknown>> {
        let (mut flags, mut x, mut y) = (
            self.as_mut().read_u8().await?,
            self.as_mut().read_u8().await?,
            self.as_mut().read_u8().await?);
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
    async fn read_char(mut self: Pin<&mut Self>) -> io::Result<char> {
        let b1 = self.as_mut().read_u8().await?;
        let mut buf = [b1, 0, 0, 0];
        let w = utf8_char_width(b1);
        if w == 0 {
            return Ok(b1 as char);
        }
        let slice = &mut buf[1..w];
        self.as_mut().project().inner.read_exact(slice).await?;
        let s = std::str::from_utf8(&mut buf[0..w]).map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?;
        Ok(s.chars().exactly_one().unwrap())
    }
    async fn peek(mut self: Pin<&mut Self>) -> io::Result<Option<u8>> {
        let mut this = self.as_mut().project();
        let buf = this.inner.fill_buf().await?;
        if buf.len() == 0 {
            return Ok(None);
        }
        Ok(Some(buf[0]))
    }
    async fn read_while(mut self: Pin<&mut Self>, mut pred: impl FnMut(u8) -> bool) -> io::Result<Vec<u8>> {
        let mut result = vec![];
        while let Some(c) = self.as_mut().peek().await? {
            if !pred(c) {
                break;
            }
            result.push(c);
            self.as_mut().project().inner.consume(1);
        }
        Ok(result)
    }
    async fn read_number(mut self: Pin<&mut Self>) -> io::Result<Option<isize>> {
        let vec = self.as_mut().read_while(|c| c.is_ascii_digit()).await?;
        if vec.len() == 0 {
            return Ok(None);
        } else {
            return Ok(Some(
                std::str::from_utf8(&vec)
                    .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?.parse()
                    .map_err(|e| io::Error::new(ErrorKind::InvalidData, e))?));
        }
    }
    async fn read_numbers(mut self: Pin<&mut Self>) -> io::Result<Vec<isize>> {
        let mut result = vec![];
        while let Some(n) = self.as_mut().read_number().await? {
            result.push(n);
            if self.as_mut().peek().await? == Some(b';') {
                self.as_mut().project().inner.consume(1);
                continue;
            } else {
                break;
            }
        }
        Ok(result)
    }
    async fn read_u8(self: Pin<&mut Self>) -> io::Result<u8> {
        let mut buf = [0u8];
        self.project().inner.read_exact(&mut buf).await?;
        Ok(buf[0])
    }
}

// https://tools.ietf.org/html/rfc3629
static UTF8_CHAR_WIDTH: [u8; 256] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x1F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x3F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x5F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x7F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0x9F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0xBF
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    2, // 0xDF
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // 0xEF
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xFF
];

/// Given a first byte, determines how many bytes are in this UTF-8 character.
#[inline]
pub fn utf8_char_width(b: u8) -> usize {
    UTF8_CHAR_WIDTH[b as usize] as usize
}