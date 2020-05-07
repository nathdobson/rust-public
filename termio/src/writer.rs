use crate::screen::{Style, Screen, LineSetting, advance, Rune};
use crate::output::*;
use crate::output::DoubleHeightTop;
use crate::color::Color;
use std::collections::BTreeSet;
use util::io::{PipelineWriter, ProfiledWrite};
use crate::util::io::SafeWrite;
use std::cmp::Ordering;
use util::profile::Profile;
use std::path::PathBuf;
use rand::thread_rng;
use rand::Rng;
use std::fmt::Display;

pub struct TermWriter {
    cursor: (isize, isize),
    style: Style,
    background: Style,
    inner: PipelineWriter,
    screen: Screen,
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
    pub fn new(inner: PipelineWriter) -> Self {
        let style = Style::default();
        let mut this = TermWriter {
            cursor: (1, 1),
            style: style,
            background: style,
            inner: inner,
            screen: Screen::new(),
        };

        swrite!(this.inner, "{}", AllMotionTrackingEnable);
        swrite!(this.inner, "{}", FocusTrackingEnable);
        swrite!(this.inner, "{}", ReportTextAreaSize);
        swrite!(this.inner, "{}", AlternateEnable);
        swrite!(this.inner, "{}", CursorHide);
        swrite!(this.inner, "{}", CursorPosition(1,1));
        swrite!(this.inner, "{}", Background(Color::Default));
        swrite!(this.inner, "{}", SingleWidthLine);
        this
    }
    pub fn close(&mut self) {
        swrite!(self.inner, "{}", AllMotionTrackingDisable);
        swrite!(self.inner, "{}", FocusTrackingDisable);
        swrite!(self.inner, "{}", AlternateDisable);
        swrite!(self.inner, "{}", CursorShow);
    }
    pub fn move_cursor(&mut self, x: isize, y: isize) {
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
            self.move_cursor(self.cursor.0, y);
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
    pub fn render(&mut self, screen: &Screen, background: &Style) {
        if &self.background != background {
            self.set_style(background);
            self.clear();
        }
        for (&y, row) in screen.rows.iter() {
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
    pub fn flush(&mut self) {
        self.inner.safe_flush();
    }
}

