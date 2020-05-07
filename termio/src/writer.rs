use crate::screen::{Style, Screen, LineSetting, advance};
use crate::safe_write::SafeWrite;
use crate::output::*;
use util::io::FullBufWriter;
use crate::output::DoubleHeightTop;
use crate::color::Color;
use std::collections::BTreeSet;

pub struct TermWriter {
    cursor: (isize, isize),
    style: Style,
    screen: Screen,
    inner: FullBufWriter<Box<dyn SafeWrite>>,
}

fn is_safe_write<T: SafeWrite + ?Sized>() {}

fn foo() {
    is_safe_write::<FullBufWriter<Box<dyn SafeWrite>>>()
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Debug, Hash)]
enum GuiState {
    Starting,
    Painting,
    Closing,
    Closed,
}

impl TermWriter {
    pub fn new(inner: Box<dyn SafeWrite + 'static>) -> Self {
        let mut this = TermWriter {
            cursor: (1, 1),
            style: Style {
                background: Default::default(),
                foreground: Default::default(),
                line_setting: Default::default(),
            },
            screen: Screen::new(),
            inner: FullBufWriter::new(inner),
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
        self.cursor = (x, y);
        swrite!(self.inner, "{}", CursorPosition(x, y));
    }
    pub fn set_line_setting(&mut self, setting: LineSetting) {
        self.style.line_setting = setting;
        match setting {
            LineSetting::Normal => swrite!(self.inner, "{}", SingleWidthLine),
            LineSetting::DoubleHeightTop => swrite!(self.inner, "{}", DoubleHeightTop),
            LineSetting::DoubleHeightBottom => swrite!(self.inner, "{}", DoubleHeightBottom),
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
        self.cursor.0 += advance(text, self.style.line_setting);
    }
    pub fn render(&mut self, screen: &Screen) {
        swrite!(self.inner, "{}", CursorSave);
        for (y, row) in screen.rows.iter().enumerate() {
            if y == 0 {
                continue;
            }
            let y = y as isize;
            self.move_cursor(1, y);
            let settings: BTreeSet<_> = row.values().map(|rune| {
                rune.style.line_setting
            }).collect();
            if settings.len() > 1 {
                eprintln!("Could not render {} because of {:?}", y, settings);
            }
            let setting = settings.iter().last().cloned().unwrap_or(LineSetting::Normal);
            self.set_line_setting(setting);
            for (range, rune) in row.iter() {
                let x = *range.start as isize;
                if x < self.cursor.0 {
                    println!("Hidden rune \"{:?}\"\n", rune);
                }
                if x > self.cursor.0 {
                    self.move_cursor(x, y);
                }
                assert_eq!(x, self.cursor.0);
                self.set_style(&rune.style);
                self.write(range.end - range.start, &rune.text);
            }
        }
        swrite!(self.inner, "{}", CursorRestore);
    }
}
