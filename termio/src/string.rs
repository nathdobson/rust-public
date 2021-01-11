use crate::screen::Style;
use std::fmt::{Display, Arguments};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::{iter, fmt};
use itertools::Itertools;
use crate::color::Color;
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct __UseDefaultDefaultToBuildStyleOption {}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct StyleOption {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    #[doc(hidden)]
    #[serde(skip)]
    pub __use_default_default_to_build_style_option__: __UseDefaultDefaultToBuildStyleOption,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct StyleString {
    vector: Vec<(StyleOption, String)>
}

pub struct StyleFormatter<'a> {
    writer: &'a mut dyn StyleWrite,
    style: StyleOption,
}

pub trait StyleWrite {
    fn style_write(&mut self, style: StyleOption, value: &dyn Display);
}

pub trait StyleFormat {
    fn style_format(&self, formatter: StyleFormatter);
}

impl StyleOption {
    pub fn or(self, other: StyleOption) -> StyleOption {
        StyleOption {
            foreground: self.foreground.or(other.foreground),
            background: self.background.or(other.background),
            __use_default_default_to_build_style_option__: __UseDefaultDefaultToBuildStyleOption {},
        }
    }
    pub fn unwrap_or(self, other: Style) -> Style {
        Style {
            foreground: self.foreground.unwrap_or(other.foreground),
            background: self.background.unwrap_or(other.background),
            __use_default_default_to_build_style__: Default::default(),
        }
    }
}

impl StyleString {
    pub fn new() -> StyleString {
        StyleString {
            vector: vec![],
        }
    }
}

impl<'a> StyleFormatter<'a> {
    pub fn new(writer: &'a mut dyn StyleWrite) -> Self {
        StyleFormatter {
            writer,
            style: StyleOption::default(),
        }
    }
    pub fn push<'b>(&'b mut self) -> StyleFormatter<'b> {
        StyleFormatter {
            writer: self.writer,
            style: self.style,
        }
    }
    pub fn push_style<'b>(&'b mut self, style: StyleOption) -> StyleFormatter<'b> {
        StyleFormatter {
            writer: self.writer,
            style: style.or(self.style),
        }
    }
    pub fn write_fmt(&mut self, args: Arguments) {
        self.writer.style_write(self.style, &args);
    }
}

impl Default for StyleOption {
    fn default() -> Self {
        StyleOption {
            foreground: None,
            background: None,
            __use_default_default_to_build_style_option__: __UseDefaultDefaultToBuildStyleOption {},
        }
    }
}

impl StyleWrite for StyleString {
    fn style_write(&mut self, style: StyleOption, value: &dyn Display) {
        if !self.vector.last().iter().any(|(old, _)| old == &style) {
            self.vector.push((style, String::new()));
        }
        write!(self.vector.last_mut().unwrap().1, "{}", value).unwrap();
    }
}

impl StyleFormat for StyleString {
    fn style_format(&self, mut formatter: StyleFormatter) {
        for (style, string) in self.vector.iter() {
            write!(formatter.push_style(*style), "{}", string);
        }
    }
}

pub trait StyleFormatExt: StyleFormat {
    fn to_style_string(&self) -> StyleString {
        let mut result = StyleString::new();
        self.style_format(StyleFormatter::new(&mut result));
        result
    }
}

impl<T: StyleFormat> StyleFormatExt for T {}


impl<'a, T: StyleFormat + ?Sized> StyleFormat for &'a T {
    fn style_format(&self, formatter: StyleFormatter) {
        (*self).style_format(formatter);
    }
}

pub struct StyleConcat<I>(I) where I: Iterator + Clone, I::Item: StyleFormat;

impl<'a, I> StyleFormat for StyleConcat<I> where I: Iterator + Clone, I::Item: StyleFormat {
    fn style_format(&self, mut formatter: StyleFormatter) {
        for x in self.0.clone() {
            x.style_format(formatter.push());
        }
    }
}

pub struct StyleJoin<'a, I>(I, &'a dyn StyleFormat) where I: Iterator + Clone, I::Item: StyleFormat;

pub trait IteratorExt: Iterator where Self::Item: StyleFormat {
    fn style_join(self, sep: &dyn StyleFormat) -> StyleJoin<Self> where Self: Sized + Clone {
        StyleJoin(self, sep)
    }
    fn style_concat(self) -> StyleConcat<Self> where Self: Sized + Clone {
        StyleConcat(self)
    }
}

impl<I> IteratorExt for I where I: Iterator + Clone, I::Item: StyleFormat {}

impl<'a, I> StyleFormat for StyleJoin<'a, I> where I: Iterator + Clone, I::Item: StyleFormat {
    fn style_format(&self, mut formatter: StyleFormatter) {
        let mut it = self.0.clone().peekable();
        while let Some(next) = it.next() {
            next.style_format(formatter.push());
            if it.peek().is_some() {
                self.1.style_format(formatter.push());
            }
        }
    }
}

macro_rules! style_format_display {
    ($($xs:ty)*) => {
        $(
            impl StyleFormat for $xs {
                fn style_format(&self, mut formatter: StyleFormatter) {
                    write!(formatter, "{}", self);
                }
            }
        )*
    }
}

style_format_display!(i8 u8 i16 u16 i32 u32 i64 u64 i128 u128 isize usize str String char);
