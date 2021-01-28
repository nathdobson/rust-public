use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::Display;
use std::path::PathBuf;

use rand::Rng;
use rand::thread_rng;
use util::io::{PipelineWriter, ProfiledWrite};
use util::profile::Profile;

use crate::color::Color;
use crate::output::*;
use crate::output::DoubleHeightTop;
use crate::screen::{LineSetting, Rune, Screen, Style, Row};
use crate::util::io::SafeWrite;
use util::rect::Rect;
use itertools::Itertools;
use arrayvec::ArrayString;
use std::borrow::Borrow;
use crate::advance::advance_of_grapheme;

#[derive(Debug)]
pub struct TermWriter {
    cursor: (isize, isize),
    style: Style,
    inner: Vec<u8>,
    screen: Screen,
    enabled: bool,
    bounds: Rect,
    get_text_size_count: usize,
}

pub fn move_cursor_raw((x1, y1): (isize, isize), (x2, y2): (isize, isize)) -> impl Display {
    AsDisplay(move |f| {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let only_x =
            if dx == 0 {
                Some("")
            } else if x2 == 1 {
                Some("\r")
            } else {
                None
            };
        let only_y =
            if dy == 0 {
                Some("")
            } else if dy == 1 {
                Some("\n")
            } else {
                None
            };
        if only_x.is_none() && only_y.is_none() {
            write!(f, "{}", CursorPosition(x2 as usize, y2 as usize))?;
            return Ok(());
        }
        if let Some(only_x) = only_x {
            write!(f, "{}", only_x)?;
        } else if dx < 0 {
            write!(f, "{}", MoveLeft(-dx as usize))?;
        } else if dx > 0 {
            write!(f, "{}", MoveRight(dx as usize))?;
        }
        if let Some(only_y) = only_y {
            write!(f, "{}", only_y)?;
        } else if dy < 0 {
            write!(f, "{}", MoveUp(-dy as usize))?;
        } else if dy > 0 {
            write!(f, "{}", MoveDown(dy as usize))?;
        }
        Ok(())
    })
}

impl TermWriter {
    pub fn new() -> Self {
        let style = Style::default();
        TermWriter {
            cursor: (1, 1),
            style: style,
            inner: vec![],
            screen: Screen::new((0, 0), style),
            enabled: false,
            bounds: Rect::from_position_size((1, 1), (1000, 1000)),
            get_text_size_count: 0,
        }
    }
    pub fn buffer(&mut self) -> &mut Vec<u8> {
        &mut self.inner
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            if enabled {
                swrite!(self.inner, "{}", AllMotionTrackingEnable);
                swrite!(self.inner, "{}", FocusTrackingEnable);
                swrite!(self.inner, "{}", AlternateEnable);
                swrite!(self.inner, "{}", CursorHide);
                swrite!(self.inner, "{}", CursorPosition(1,1));
                swrite!(self.inner, "{}", Background(Color::Default));
                swrite!(self.inner, "{}", SingleWidthLine);
            } else {
                swrite!(self.inner, "{}", AllMotionTrackingDisable);
                swrite!(self.inner, "{}", FocusTrackingDisable);
                swrite!(self.inner, "{}", AlternateDisable);
                swrite!(self.inner, "{}", CursorShow);
            }
        }
    }
    pub fn get_text_size(&mut self) {
        self.get_text_size_count += 1;
        swrite!(self.inner, "{}", ReportTextAreaSize);
    }
    pub fn get_text_size_count(&self) -> usize {
        self.get_text_size_count
    }
    pub fn get_cursor_position(&mut self) {
        swrite!(self.inner,"{}",ReportCursorPosition);
    }
    pub fn repair(&mut self) {
        swrite!(self.inner, "{}", CursorPosition(self.cursor.0 as usize, self.cursor.1 as usize));
        return;
    }
    pub fn move_cursor(&mut self, x: isize, y: isize) {
        assert!(x > 0);
        if self.cursor == (x, y) {
            return;
        }
        let string = move_cursor_raw(self.cursor, (x, y));
        swrite!(self.inner, "{}", string);
        self.cursor = (x, y);
    }
    pub fn set_line_setting(&mut self, y: isize, setting: LineSetting) {
        let old = &mut self.screen.rows[y as usize].line_setting;
        if *old != setting {
            *old = setting;
            self.move_cursor(1, y);
            match setting {
                LineSetting::Normal => swrite!(self.inner, "{}", SingleWidthLine),
                LineSetting::DoubleHeightTop => swrite!(self.inner, "{}", DoubleHeightTop),
                LineSetting::DoubleHeightBottom => swrite!(self.inner, "{}", DoubleHeightBottom),
            }
        }
    }
    pub fn set_style(&mut self, style: &Style) {
        if style.foreground != self.style.foreground {
            swrite!(self.inner, "{}", Foreground(style.foreground));
            self.style.foreground = style.foreground;
        }
        if style.background != self.style.background {
            swrite!(self.inner, "{}", Background(style.background));
            self.style.background = style.background;
        }
    }
    pub fn write(&mut self, length: isize, text: &str) {
        swrite!(self.inner, "{}", text);
        let row = &mut self.screen.rows[self.cursor.1 as usize];
        row.write(self.cursor.0, length, text, self.style);
        self.cursor.0 += length;
        let max =
            if row.line_setting == LineSetting::Normal {
                self.bounds.xs().end
            } else {
                (self.bounds.xs().end + 10) / 2
            };
        if self.cursor.0 > max {
            self.cursor.0 = max;
        }
    }
    pub fn delete_line(&mut self) {
        let def = self.screen.default_rune();
        self.screen.rows[self.cursor.1 as usize].runes[self.cursor.0 as usize..].fill(def);
        swrite!(self.inner, "{}", DeleteLineRight);
    }
    pub fn clear(&mut self) {
        self.screen.background = self.style;
        swrite!(self.inner, "{}{}", EraseAll, CursorPosition(1,1));
        self.screen.clear();
        self.cursor = (1, 1);
    }
    pub fn set_bounds(&mut self, bounds: Rect) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.clear();
        }
    }
    pub fn render(&mut self, screen: &Screen) {
        if screen.title != self.screen.title {
            swrite!(self.inner, "{}", WindowTitle(&screen.title));
            self.screen.title.clone_from(&screen.title);
        }
        if screen.background != self.screen.background || screen.size() != self.screen.size() {
            self.set_style(&screen.background);
            self.screen = Screen::new(screen.size(), screen.background);
            self.clear();
        }

        for y in 1..screen.rows.len() {
            if screen.rows[y] == self.screen.rows[y] {
                continue;
            }
            self.set_line_setting(y as isize, screen.rows[y].line_setting);
            for x in 1..screen.rows[y].runes.len() {
                let rune = &screen.rows[y].runes[x];
                if self.screen.rows[y].runes[x] != *rune {
                    self.move_cursor(x as isize, y as isize);
                    self.set_style(&rune.style);
                    self.write(advance_of_grapheme(&rune.text), &rune.text);
                }
            }
            //TODO empty line optimization
        }
    }
}

