mod lexer;
mod parser;
mod printer;
mod path;

use std::collections::hash_map::DefaultHasher;
use std::fmt::{Display, Formatter, Debug};
use std::{fmt, mem, iter, array};
use std::hash::{Hash, Hasher};
use itertools::Itertools;

use termio::color::Color;
use termio::output::{Background, Foreground};
use either::Either;
use either::Either::{Left, Right};
use std::convert::TryFrom;
use std::fmt::Write;
use std::ffi::c_void;
use crate::remangle::parser::{ParseError, ParseResult};
use std::collections::HashMap;
use crate::remangle::printer::{MAX_QUIET, QUIET_REMOVE_VERSION};
use crate::remangle::printer::Printer;
use lazy_static::lazy_static;
use std::sync::Mutex;
use crate::remangle::parser::Parser;
use crate::remangle::path::Path;

pub fn remangle(input: &str) -> String {
    match Path::parse(input) {
        Ok(path) => path.path_to_string(QUIET_REMOVE_VERSION, 100),
        Err(_) => format!("[unparsed]::{}", input)
    }
}

lazy_static! {
    static ref CACHE: Mutex<HashMap<usize, &'static [&'static str]>> = Mutex::new(HashMap::new());
}

pub fn resolve_remangle(addr: *mut c_void) -> &'static [&'static str] {
    let mut lock = CACHE.lock().unwrap();
    let map: &mut HashMap<usize, &'static [&'static str]> = &mut *lock;
    map.entry(addr as usize).or_insert_with(|| resolve_remangle_inner(addr))
}

fn resolve_remangle_inner(addr: *mut c_void) -> &'static [&'static str] {
    let mut symbols = vec![];
    backtrace::resolve(addr, |symbol| {
        let mut line = String::new();
        if let Some(name) = symbol.name() {
            write!(&mut line, "{}", remangle(&name.to_string())).unwrap();
            if let Some(filename) = symbol.filename() {
                let filename =
                    filename.to_str().unwrap()
                        .split("src/").last().unwrap()
                        .split("examples/").last().unwrap();
                if let Some(lineno) = symbol.lineno() {
                    if let Some(colno) = symbol.colno() {
                        write!(&mut line, " ({}:{}:{})", filename, lineno, colno).unwrap();
                    } else {
                        write!(&mut line, " ({}:{})", filename, lineno).unwrap();
                    }
                } else {
                    write!(&mut line, " ({})", filename).unwrap();
                }
            }
        }
        symbols.push(Box::leak(line.into_boxed_str()) as &str);
    });
    Box::leak(symbols.into_boxed_slice())
}


