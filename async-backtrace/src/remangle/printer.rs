use crate::remangle::path::{PathSegment, Path};
use std::fmt::{Write, Arguments, write};
use std::fmt;

pub struct Printer<W: Write> {
    quiet: usize,
    output: W,
}

pub const QUIET_REMOVE_VERSION: usize = 1;
pub const QUIET_SHORTEN_ANON: usize = 2;
pub const QUIET_REMOVE_MODULE: usize = 3;
pub const QUIET_REMOVE_AS_TRAIT: usize = 4;
pub const QUIET_REMOVE_PARAMETERS: usize = 5;
pub const MAX_QUIET: usize = 5;

const ELLIPSIS: &'static str = "…";


impl<W: Write> Printer<W> {
    pub fn new(quiet: usize, output: W) -> Self {
        Printer { quiet, output }
    }
    pub fn print_segment(&mut self, segment: &PathSegment) -> fmt::Result {
        match segment {
            PathSegment::Ident { name, version, turbofish, tys } => {
                write!(self, "{}", name)?;
                if self.quiet < QUIET_REMOVE_VERSION {
                    if let Some(version) = version {
                        write!(self, "[{}]", version)?;
                    }
                }
                if !tys.is_empty() {
                    if *turbofish {
                        write!(self, "::")?;
                    }
                    write!(self, "<")?;
                    if self.quiet < QUIET_REMOVE_PARAMETERS {
                        for ty in Iterator::intersperse(tys.iter().map(Some), None) {
                            if let Some(ty) = ty {
                                self.print(ty)?;
                            } else {
                                write!(self, ", ")?;
                            }
                        }
                    } else {
                        write!(self, "{}", ELLIPSIS)?;
                    }
                    write!(self, ">")?;
                }
            }
            PathSegment::ImplFor { trait_for, for_ty } => {
                write!(self, "<impl ")?;
                self.print(trait_for)?;
                write!(self, " for ")?;
                self.print(for_ty)?;
                write!(self, ">")?;
            }
            PathSegment::Ty { ty } => {
                write!(self, "<")?;
                self.print(ty)?;
                write!(self, ">")?;
            }
            PathSegment::As { ty, as_trait } => {
                write!(self, "<")?;
                self.print(ty)?;
                if self.quiet < QUIET_REMOVE_AS_TRAIT {
                    write!(self, " as ")?;
                    self.print(as_trait)?;
                }
                write!(self, ">")?;
            }
            PathSegment::Pointy { raw, mutable, ty } => {
                write!(self, "{}", if *raw {
                    if *mutable { "*mut " } else { "*const " }
                } else {
                    if *mutable { "&mut " } else { "&" }
                })?;
                self.print(ty)?;
            }
            PathSegment::Tuple { tys } => {
                write!(self, "(")?;
                for ty in Iterator::intersperse(tys.iter().map(Some), None) {
                    if let Some(ty) = ty {
                        self.print(ty)?;
                    } else {
                        write!(self, ", ")?;
                    }
                }
                if tys.len() == 1 {
                    write!(self, ",")?;
                }
                write!(self, ")")?;
            }
            PathSegment::VTable { vtable } => {
                if self.quiet < QUIET_SHORTEN_ANON {
                    write!(self, "{{shim:vtable#{}}}", vtable)?;
                } else {
                    write!(self, "𝓿{}", vtable)?;
                }
            }
            PathSegment::Closure { closure } => {
                if self.quiet < QUIET_SHORTEN_ANON {
                    write!(self, "{{closure#{}}}", closure)?;
                } else {
                    write!(self, "λ{}", closure)?;
                }
            }
            PathSegment::FnPtr { tys, output } => {
                write!(self, "fn(")?;
                for ty in Iterator::intersperse(tys.iter().map(Some), None) {
                    if let Some(ty) = ty {
                        self.print(ty)?;
                    } else {
                        write!(self, ", ")?;
                    }
                }
                write!(self, ")")?;
                if let Some(output) = output {
                    write!(self, " -> ")?;
                    self.print(output)?;
                }
            }
            PathSegment::Array { ty, length, length_ty } => {
                write!(self, "[")?;
                self.print(ty)?;
                if let Some(length) = length {
                    write!(self, "; {}", length)?;
                    if let Some(length_ty) = length_ty {
                        write!(self, ": ")?;
                        self.print(length_ty)?;
                    }
                }
                write!(self, "]")?;
            }
        }
        Ok(())
    }
    pub fn print(&mut self, path: &Path) -> fmt::Result {
        let mut start = None;
        if self.quiet >= QUIET_REMOVE_MODULE {
            for (index, segment) in path.segments.iter().enumerate() {
                match segment {
                    PathSegment::Ident { name, .. } => {
                        start = Some(index);
                        if name.chars().next().unwrap().is_ascii_uppercase() {
                            break;
                        }
                    }
                    PathSegment::Closure { .. } | PathSegment::VTable { .. } => {}
                    _ => {
                        start = Some(index);
                        break;
                    }
                }
            }
        }
        let start = start.unwrap_or(0);
        if start > 0 {
            write!(self, "{}", ELLIPSIS)?;
        }
        let mut end = path.segments.len();
        if self.quiet >= QUIET_REMOVE_VERSION {
            match path.segments.last() {
                Some(PathSegment::Ident { name, version, turbofish, tys })
                => {
                    if let Some(name) = name.strip_prefix("h") {
                        if name.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) && version.is_none() && !*turbofish && tys.is_empty() {
                            end -= 1;
                        }
                    }
                }
                _ => {}
            }
        }
        for segment in Iterator::intersperse(path.segments[start..end].iter().map(Some), None) {
            if let Some(segment) = segment {
                self.print_segment(segment)?;
            } else {
                write!(self, "::")?;
            }
        }
        if end < path.segments.len() {
            write!(self, "{}", ELLIPSIS)?;
        }
        Ok(())
    }
}

impl<W: Write> Write for Printer<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result { self.output.write_str(s) }

    fn write_char(&mut self, c: char) -> fmt::Result { self.output.write_char(c) }

    fn write_fmt(self: &mut Self, args: Arguments<'_>) -> fmt::Result { self.output.write_fmt(args) }
}


#[test]
fn test_printer() {
    use crate::remangle::path::EXAMPLES;
    for example in EXAMPLES {
        println!();
        println!("{}", example);
        let path = Path::parse(example).unwrap();
        for quiet in 0..=MAX_QUIET {
            let mut output = String::new();
            let mut printer = Printer::new(quiet, &mut output);
            printer.print(&path).unwrap();
            println!("{}", output);
        }
    }
}
