use crate::string::StyleString;
use std::collections::HashSet;
use crate::screen::LineSetting;
use crate::advance::advance_of_style_string;

pub struct ImageRow {
    pub string: StyleString,
    pub line_setting: LineSetting,
}

pub struct Image {
    rows: Vec<ImageRow>,
    size: (isize, isize),
}

impl Image {
    pub fn new(rows: Vec<ImageRow>) -> Self {
        let width = advance_of_style_string(&rows[0].string) * match rows[0].line_setting {
            LineSetting::Normal => 1,
            LineSetting::DoubleHeightTop => 2,
            LineSetting::DoubleHeightBottom => 2,
        };
        let height = rows.len() as isize;
        Image {
            rows,
            size: (width, height),
        }
    }
    pub fn size(&self) -> (isize, isize) {
        self.size
    }
    pub fn rows(&self) -> &[ImageRow] {
        &self.rows
    }
}