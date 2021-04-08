use std::collections::hash_map::DefaultHasher;
use std::fmt::{Display, Formatter};
use std::{fmt, mem};
use std::hash::{Hash, Hasher};

use itertools::Itertools;

use termio::color::Color;
use termio::output::{Background, Foreground};
use either::Either;
use either::Either::{Left, Right};
use std::convert::TryFrom;
use std::fmt::Write;

#[derive(Debug)]
pub struct ParseError(&'static str);

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "backtrace parse error")
    }
}

struct Parser<'a> {
    string: &'a str,
}

const STAGE_REMOVE_MODULE: usize = 1;

#[derive(Debug, Clone)]
enum PathSegment<'a> {
    Inherent {
        path: Path<'a>,
        as_trait: Option<Path<'a>>,
    },
    Ident {
        name: &'a str,
        params: Vec<Path<'a>>,
    },
}

#[derive(Debug, Clone)]
pub struct Path<'a> {
    segments: Vec<PathSegment<'a>>,
}

impl<'a> Parser<'a> {
    fn new(string: &'a str) -> Self {
        Parser { string }
    }
    fn read(&mut self, prefix: &str) -> bool {
        if let Some(string) = self.string.strip_prefix(prefix) {
            self.string = string;
            true
        } else {
            false
        }
    }
    fn can_read(&self, prefix: &str) -> bool {
        self.string.starts_with(prefix)
    }
    fn read_ident(&mut self) -> Option<&'a str> {
        let mut chars = self.string.chars();
        let mut new_string;
        let mut depth = 0;
        loop {
            new_string = chars.as_str();
            if let Some(c) = chars.next() {
                if c == '[' || c == '(' || c == '{' {
                    depth += 1;
                } else if c == ']' || c == ')' || c == '}' {
                    if depth == 0 {
                        break;
                    } else {
                        depth -= 1;
                    }
                } else if c == ',' || c == ':' || c == '<' || c == '>' {
                    if depth == 0 {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        let result = &self.string[..self.string.len() - new_string.len()];
        self.string = new_string;
        if result.len() == 0 {
            None
        } else {
            Some(result)
        }
    }
    fn finish(&self) -> Result<(), ParseError> {
        if self.string.is_empty() {
            Ok(())
        } else {
            Err(ParseError("Unused tokens"))
        }
    }
    fn parse_segment(&mut self) -> Result<Option<PathSegment<'a>>, ParseError> {
        if self.string.len() == 0 {
            return Ok(None);
        }
        if self.can_read(" as ") || self.can_read(">") || self.can_read(",") {
            return Ok(None);
        }
        if self.read("<") {
            let path = self.parse_path()?;
            let mut as_trait = None;
            if self.read(" as ") {
                as_trait = Some(self.parse_path()?);
            }
            if !self.read(">") {
                return Err(ParseError("inherent bad ending"));
            }
            return Ok(Some(PathSegment::Inherent { path, as_trait }));
        } else if let Some(name) = self.read_ident() {
            let mut params = vec![];
            if self.read("::<") || self.read("<") {
                loop {
                    params.push(self.parse_path()?);
                    if self.read(", ") || self.read(",") {
                        continue;
                    } else if self.read(">") {
                        break;
                    } else {
                        println!("{:?}", self.string);
                        return Err(ParseError("params ending incorrect"));
                    }
                }
            }
            return Ok(Some(PathSegment::Ident { name, params }));
        } else {
            println!("{}", self.string);
            return Err(ParseError("No valid segment prefix"));
        }
    }
    fn parse_path(&mut self) -> Result<Path<'a>, ParseError> {
        let mut result = Path { segments: vec![] };
        if let Some(segment) = self.parse_segment()? {
            result.segments.push(segment);
            while self.read("::") {
                result.segments.push(self.parse_segment()?.ok_or(ParseError("missing ::"))?);
            }
        }
        Ok(result)
    }
}

impl<'a> PathSegment<'a> {
    fn match_ident<'b>(&'b self, ident: &str) -> Option<()> where 'a: 'b {
        match self {
            PathSegment::Ident { name, params } if params.is_empty() && *name == ident => Some(()),
            _ => None
        }
    }
    fn match_ident_arg<'b>(&'b self, ident: &str) -> Option<&'b Path<'a>> where 'a: 'b {
        match self {
            PathSegment::Ident { name, params } if *name == ident => {
                params.iter().exactly_one().ok()
            }
            _ => None
        }
    }
    // fn match_inherent<'b>(&'b self) -> Option<&'b Path<'a>> where 'a: 'b {
    //     match self {
    //         PathSegment::Inherent { path, as_trait: None } => Some(path),
    //         _ => None
    //     }
    // }
    fn match_trait<'b>(&'b self) -> Option<(&'b Path<'a>, &'b Path<'a>)> where 'a: 'b {
        match self {
            PathSegment::Inherent { path, as_trait: Some(as_trait) } => Some((path, as_trait)),
            _ => None
        }
    }
    fn compress_stage(&self, stage: usize, output: &mut String) {
        match self {
            PathSegment::Inherent { path, as_trait: None } =>
                {
                    output.push_str("<");
                    path.compress_stage(stage, output);
                    output.push_str(">");
                }

            PathSegment::Inherent { path, as_trait: Some(as_trait) } => {
                output.push_str("<");
                path.compress_stage(stage, output);
                output.push_str(",");
                as_trait.compress_stage(stage, output);
                output.push_str(">");
            }

            PathSegment::Ident { name, params } => {
                if let Some(closure_name) = name.strip_prefix("{closure#").and_then(|x| x.strip_suffix("}")) {
                    output.push_str("λ");
                    output.push_str(closure_name);
                } else {
                    output.push_str(name);
                }
                if params.len() > 0 {
                    output.push_str("<");
                    for param in params[..params.len() - 1].iter() {
                        param.compress_stage(stage, output);
                        output.push_str(",");
                    }
                    params.last().unwrap().compress_stage(stage, output);
                    output.push_str(">");
                }
            }
        };
    }
}

impl<'a> Path<'a> {
    fn parse(name: &'a str) -> Result<Self, ParseError> {
        let mut parser = Parser::new(name);
        let result = parser.parse_path()?;
        parser.finish()?;
        Ok(result)
    }
    fn remove_versions(&mut self, success: &mut bool) {
        for segment in self.segments.iter_mut() {
            match segment {
                PathSegment::Inherent { path, as_trait } => {
                    path.remove_versions(success);
                    if let Some(as_trait) = as_trait {
                        as_trait.remove_versions(success);
                    }
                }
                PathSegment::Ident { name, params } => {
                    let old_name = *name;
                    *name = name.split('[').next().unwrap();
                    if old_name != *name {
                        *success = true;
                    }
                    for param in params {
                        param.remove_versions(success)
                    }
                }
            }
        }
    }
    fn remove_hash(&mut self) {
        if self.segments.len() > 2 {
            self.segments.pop();
        }
    }

    fn match_length<'b, const C: usize>(&'b self) -> Option<[&'b PathSegment<'a>; C]> where 'a: 'b {
        <[&PathSegment; C]>::try_from(self.segments.iter().collect::<Vec<_>>().as_slice()).ok()
    }

    fn translate_generator_clone<'b>(&'b mut self) -> Option<Path<'a>> {
        let [inherent, poll] = self.match_length()?;
        let (inst, future) = inherent.match_trait()?;
        let result: &'b Path<'a>;
        {
            let [core, future1, from_generator, gen_future] = inst.match_length()?;
            core.match_ident("core")?;
            future1.match_ident("future")?;
            from_generator.match_ident("from_generator")?;
            result = gen_future.match_ident_arg("GenFuture")?;
        }
        {
            let [core, future1, future2, future3] = future.match_length()?;
            core.match_ident("core")?;
            future1.match_ident("future")?;
            future2.match_ident("future")?;
            future3.match_ident("Future")?;
        }
        poll.match_ident("poll")?;
        let mut result = result.clone();
        result.segments.push(PathSegment::Ident { name: "poll", params: vec![] });
        Some(result)
    }

    fn translate_generator<'b>(&'b mut self) -> Option<()> {
        *self = self.translate_generator_clone()?;
        Some(())
    }

    fn compress_stage(&self, stage: usize, output: &mut String) {
        let mut first_required = None;
        let mut last_name = None;
        for (index, segment) in self.segments.iter().enumerate() {
            match segment {
                PathSegment::Inherent { .. } => {
                    if first_required.is_none() {
                        first_required = Some(index);
                    }
                }
                PathSegment::Ident { name, .. } => {
                    let c = name.chars().next().unwrap();
                    if c.is_ascii_uppercase() && first_required.is_none() {
                        first_required = Some(index);
                    }
                    if c.is_ascii_alphanumeric() {
                        last_name = Some(index);
                    }
                }
            }
        }
        let mut start = 0;
        if stage >= STAGE_REMOVE_MODULE {
            if let Some(last_name) = last_name {
                if let Some(first_required) = first_required {
                    start = first_required;
                } else {
                    start = last_name;
                }
            }
        }
        if start > 0 {
            output.push_str("…");
        }
        for segment in Iterator::intersperse(self.segments[start..].iter().map(Some), None) {
            if let Some(segment) = segment {
                segment.compress_stage(stage, output);
            } else {
                output.push_str("::");
            }
        }
    }

    fn compress(&self, max_length: usize) -> String {
        let mut result = String::new();
        for i in 0..3 {
            result.clear();
            self.compress_stage(i, &mut result);
            if result.len() <= max_length {
                return result;
            }
        }
        if let Some((b, _)) = result.char_indices().nth(max_length - 1) {
            result.truncate(b);
            result.push_str("…");
        }
        result
    }
}

// impl<'a> Display for PathSegment<'a> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         match self {
//             PathSegment::Inherent { path, as_trait: None } =>
//                 write!(f, "<{}>", path)?,
//
//             PathSegment::Inherent { path, as_trait: Some(as_trait) } =>
//                 write!(f, "<{} as {}>", path, as_trait)?,
//
//             PathSegment::Ident { name, params } => {
//                 write!(f, "{}", name)?;
//                 if params.len() > 0 {
//                     write!(f, "<")?;
//                     for param in params[..params.len() - 1].iter() {
//                         write!(f, "{},", param)?;
//                     }
//                     write!(f, "{}", params.last().unwrap())?;
//                     write!(f, ">")?;
//                 }
//             }
//         }
//         Ok(())
//     }
// }

impl<'a> Display for Path<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.compress(100))
        // for x in Iterator::intersperse(self.segments.iter().map(Left), Right("::")) {
        //     write!(f, "{}", x)?;
        // }
        // Ok(())
    }
}

// fn hash_color(s: &str) -> Color {
//     let mut hasher = DefaultHasher::new();
//     s.hash(&mut hasher);
//     let hash = hasher.finish();
//     let r = ((hash >> 32) & ((1 << 16) - 1)) * 5 / (1 << 16);
//     let g = ((hash >> 16) & ((1 << 16) - 1)) * 5 / (1 << 16);
//     let b = ((hash >> 0) & ((1 << 16) - 1)) * 5 / (1 << 16);
//     Color::RGB666(r as u8, g as u8, b as u8)
// }

pub fn remangle(s: &str) -> String {
    let mut parsed;
    match Path::parse(s) {
        Ok(p) => parsed = p,
        Err(e) => return format!("[parsed failed]::{}", s),
    }
    let mut output = String::new();
    let mut had_version = false;
    parsed.remove_versions(&mut had_version);
    if !had_version {
        parsed.remove_hash();
    }
    parsed.translate_generator();
    output.push_str(&parsed.to_string());

    output
}

#[test]
fn test_parse() {
    println!("{}", Path::parse("<aa::bb[444] as Foo::ddd>::parse").unwrap());
    println!("{}", Path::parse("<tokio[b42d84aa1fe2c576]::loom::std::unsafe_cell::UnsafeCell<core[75ee33869dcd9a7b]::option::Option<core[75ee33869dcd9a7b]::task::wake::Waker>>>::with_mut::<(), <tokio[b42d84aa1fe2c576]::sync::task::atomic_waker::AtomicWaker>::do_register<&core[75ee33869dcd9a7b]::task::wake::Waker>::{closure#1}>").unwrap());
    println!("{}", Path::parse("<async_backtrace[ac85353af3ad180c]::TRACE_WAKER_VTABLE::{closure#0} as core[75ee33869dcd9a7b]::ops::function::FnOnce<(*const (),)>>::call_once").unwrap());
    println!("{}", Path::parse("<alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once::h956be82c5a7d95d6").unwrap());
}

#[test]
fn test_compression() {
    let path = Path::parse("<tokio::loom::std::unsafe_cell::UnsafeCell<core::option::Option<core::task::wake::Waker>>>::with_mut<(),<tokio::sync::task::atomic_waker::AtomicWaker>::do_register<&core::task::wake::Waker>::{closure#1}>").unwrap();
    println!("{}", path.compress(100));
    let path = Path::parse("<std::thread::local::LocalKey<core::cell::Cell<tokio::coop::Budget>>>::try_with<tokio::coop::with_budget<core::task::poll::Poll<()>,<tokio::park::thread::CachedParkThread>::block_on<core::future::from_generator::GenFuture<async_backtrace::test::test_basic::{closure#0}>>::{closure#0}>::{closure#0},core::task::poll::Poll<()>>").unwrap();
    println!("{}", path.compress(100));
}