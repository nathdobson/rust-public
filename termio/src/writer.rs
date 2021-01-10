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
use crate::screen::{advance, LineSetting, Rune, Screen, Style, Row};
use crate::util::io::SafeWrite;
use util::rect::Rect;

pub struct TermWriter {
    cursor: (isize, isize),
    style: Style,
    background: Style,
    inner: Vec<u8>,
    screen: Screen,
    enabled: bool,
    bounds: Rect,
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
            background: style,
            inner: vec![],
            screen: Screen::new(),
            enabled: false,
            bounds: Rect::from_position_size((1, 1), (1000, 1000)),
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
        swrite!(self.inner, "{}", ReportTextAreaSize);
    }
    pub fn move_cursor(&mut self, x: isize, y: isize) {
//        if true {
//            swrite!(self.inner, "{}", CursorPosition(x as usize, y as usize));
//            self.cursor = (x, y);
//            return;
//        }
        if self.cursor == (x, y) {
            return;
        }
        let string = move_cursor_raw(self.cursor, (x, y));
        swrite!(self.inner, "{}", string);
        self.cursor = (x, y);
    }
    pub fn set_line_setting(&mut self, y: isize, setting: LineSetting) {
        let old = &mut self.screen.row(y).line_setting;
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
        let row = self.screen.row(self.cursor.1);
        row.runes.erase_and_insert(self.cursor.0..self.cursor.0 + length, Rune {
            text: text.to_string(),
            style: self.style,
        });
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
    pub fn delete_space(&mut self) {
        swrite!(self.inner, " ");
        let row = self.screen.row(self.cursor.1);
        row.runes.erase(&self.cursor.0..&(self.cursor.0 + 1));
        self.cursor.0 += 1;
    }
    pub fn delete_line(&mut self) {
        self.screen.row(self.cursor.1).runes.erase(&self.cursor.0..);
        swrite!(self.inner, "{}", DeleteLineRight);
    }
    pub fn clear(&mut self) {
        self.background = self.style;
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
    pub fn render(&mut self, screen: &Screen, background: &Style) {
        if screen.title != self.screen.title {
            swrite!(self.inner, "{}", WindowTitle(&screen.title));
            self.screen.title.clone_from(&screen.title);
        }
        if &self.background != background {
            self.set_style(background);
            self.clear();
        }
        for y in 0..=screen.rows.keys().last().cloned().unwrap_or(-1).max(self.screen.rows.keys().last().cloned().unwrap_or(-1)) {
            let default = Row::new();
            let row = screen.rows.get(&y).unwrap_or(&default);
            if row == self.screen.row(y) {
                continue;
            }
            let y = y as isize;
            self.set_line_setting(y, row.line_setting);
            let mut x = 1;
            loop {
                let new = row.runes.range(&x.clone()..).next()
                    .map(|(xs, v)| (*xs.start..*xs.end, v.clone()));
                let old = self.screen.row(y).runes.range(&x.clone()..).next()
                    .map(|(xs, v)| (*xs.start..*xs.end, v.clone()));
                if let Some((xs_new, new)) = new {
                    if let Some((xs_old, old)) = old {
                        if xs_new == xs_old && new == old {
                            x = xs_new.end;
                            continue;
                        } else if xs_old.end <= xs_new.start {
                            self.move_cursor(xs_old.start, y);
                            self.set_style(&background);
                            while self.cursor.0 < xs_old.end {
                                self.delete_space();
                            }
                            x = self.cursor.0;
                            continue;
                        }
                    }
                    self.move_cursor(xs_new.start, y);
                    self.set_style(&new.style);
                    self.write(xs_new.end - xs_new.start, &new.text);
                    x = self.cursor.0;
                } else {
                    if old.is_some() {
                        self.move_cursor(x, y);
                        self.set_style(&background);
                        self.delete_line();
                    }
                    break;
                }
            }
        }
    }
}

