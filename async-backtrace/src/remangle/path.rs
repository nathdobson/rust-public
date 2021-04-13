use std::fmt::{Formatter, Debug};
use std::fmt;
use crate::remangle::parser::{ParseResult, Parser};
use crate::remangle::printer::{MAX_QUIET, Printer};

#[cfg(test)]
pub static EXAMPLES: &[&str] =
    &[
        "<aa::bb[444] as Foo::ddd>::parse",
        "<tokio[b42d84aa1fe2c576]::loom::std::unsafe_cell::UnsafeCell<core[75ee33869dcd9a7b]::option::Option<core[75ee33869dcd9a7b]::task::wake::Waker>>>::with_mut::<(), <tokio[b42d84aa1fe2c576]::sync::task::atomic_waker::AtomicWaker>::do_register<&core[75ee33869dcd9a7b]::task::wake::Waker>::{closure#1}>", "<async_backtrace[ac85353af3ad180c]::TRACE_WAKER_VTABLE::{closure#0} as core[75ee33869dcd9a7b]::ops::function::FnOnce<(*const (),)>>::call_once",
        "<alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once::h956be82c5a7d95d6",
        "core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once::h95e11665661cf81e",
        "<<std[3f53dec8e99b1f88]::thread::Builder>::spawn_unchecked<<tokio[693d8ff8a2386e6a]::runtime::blocking::pool::Spawner>::spawn_thread::{closure#0}, ()>::{closure#0} as core[75ee33869dcd9a7b]::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}",
        "<tokio[693d8ff8a2386e6a]::runtime::basic_scheduler::Inner<tokio[693d8ff8a2386e6a]::runtime::driver::Driver>>::block_on::<core[75ee33869dcd9a7b]::pin::Pin<&mut core[75ee33869dcd9a7b]::future::from_generator::GenFuture<async_backtrace[2a950c76ff9e6286]::server::run_debug_server::{closure#1}::{closure#0}>>>::{closure#0}",
        "<fn() as core[75ee33869dcd9a7b]::ops::function::FnOnce<()>>::call_once",
        "example[8e385caf73139a18]::for_generic::<[u8; 10: usize]>",
        "example[8e385caf73139a18]::for_generic::<fn(usize) -> usize>",
        "core::ops::function::FnOnce::call_once{{vtable.shim}}::hf54ddc002259dae6",
        "tokio::sync::task::atomic_waker::AtomicWaker::do_register::{{closure}}::h9d574f6952b7301f",
    ];

pub struct Path<'a> {
    pub segments: Vec<PathSegment<'a>>,
}

pub enum PathBraces<'a> {
    UnknownVTable,
    UnknownClosure,
    VTable { vtable: &'a str },
    Closure { closure: &'a str },
}

pub enum PathSegment<'a> {
    Ident { name: &'a str, version: Option<&'a str>, braces: Option<PathBraces<'a>>, turbofish: bool, tys: Vec<Path<'a>> },
    ImplFor { trait_for: Path<'a>, for_ty: Path<'a> },
    Ty { ty: Path<'a> },
    As { ty: Path<'a>, as_trait: Path<'a> },
    Pointy { raw: bool, mutable: bool, ty: Path<'a> },
    Tuple { tys: Vec<Path<'a>> },
    FnPtr { tys: Vec<Path<'a>>, output: Option<Path<'a>> },
    Array { ty: Path<'a>, length: Option<&'a str>, length_ty: Option<Path<'a>> },
}

impl<'a> Path<'a> {
    pub fn parse(input: &'a str) -> ParseResult<Self> {
        let mut parser = Parser::new(input);
        let result = match parser.parse_path() {
            Ok(result) => result,
            Err(err) => {
                println!("Remaining {:?}", parser.lexer.collect::<Vec<_>>());
                return Err(err);
            }
        };
        parser.finish()?;
        Ok(result)
    }
    pub fn path_to_string(&self, min_quiet: usize, max_length: usize) -> String {
        let mut output = String::new();
        for quiet in min_quiet..=MAX_QUIET {
            let mut printer = Printer::new(quiet, &mut output);
            printer.print(self).unwrap();
            if let Some((truncation_point, _)) = output.char_indices().nth(max_length) {
                if quiet == MAX_QUIET {
                    output.truncate(truncation_point);
                } else {
                    output.clear();
                }
            } else {
                break;
            }
        }
        output
    }
}


impl<'a> Debug for Path<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(Iterator::intersperse(
                self.segments.iter()
                    .map(|x| x as &dyn Debug),
                &"::" as &dyn Debug)
            ).finish()
    }
}

impl<'a> Debug for PathBraces<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PathBraces::VTable { vtable } => {
                let mut list = f.debug_list();
                list.entry(&"{");
                list.entry(&"shim:vtable#");
                list.entry(vtable);
                list.entry(&"}");
                list.finish()
            }
            PathBraces::Closure { closure } => {
                let mut list = f.debug_list();
                list.entry(&"{");
                list.entry(&"closure#");
                list.entry(closure);
                list.entry(&"}");
                list.finish()
            }
            PathBraces::UnknownVTable => {
                let mut list = f.debug_list();
                list.entry(&"{");
                list.entry(&"{");
                list.entry(&"vtable.shim");
                list.entry(&"}");
                list.entry(&"}");
                list.finish()
            }
            PathBraces::UnknownClosure => {
                let mut list = f.debug_list();
                list.entry(&"{");
                list.entry(&"{");
                list.entry(&"closure");
                list.entry(&"}");
                list.entry(&"}");
                list.finish()
            }
        }
    }
}

impl<'a> Debug for PathSegment<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PathSegment::As { ty, as_trait } => {
                f.debug_list()
                    .entry(&"<")
                    .entry(ty)
                    .entry(&" as ")
                    .entry(as_trait)
                    .entry(&">")
                    .finish()
            }
            PathSegment::ImplFor { trait_for, for_ty } => {
                f.debug_list()
                    .entry(&"<impl ")
                    .entry(trait_for)
                    .entry(&" for ")
                    .entry(for_ty)
                    .entry(&">")
                    .finish()
            }
            PathSegment::Ty { ty } => {
                f.debug_list()
                    .entry(&"<")
                    .entry(ty)
                    .entry(&">")
                    .finish()
            }
            PathSegment::Ident { name, version, braces, turbofish, tys } => {
                let mut list = f.debug_list();
                list.entry(name);
                if let Some(version) = version {
                    list.entry(&"[")
                        .entry(version)
                        .entry(&"]");
                }
                if !tys.is_empty() {
                    if *turbofish {
                        list.entry(&"::");
                    }
                    list.entry(&"<")
                        .entries(Iterator::intersperse(tys.iter().map(|x| x as &dyn Debug), &","))
                        .entry(&">");
                }
                list.finish()
            }
            PathSegment::Pointy { raw, mutable, ty } => {
                f.debug_list()
                    .entry(if *raw {
                        if *mutable { &"*mut " } else { &"*const " }
                    } else {
                        if *mutable { &"&mut " } else { &"&" }
                    })
                    .entry(ty)
                    .finish()
            }
            PathSegment::Tuple { tys } => {
                let mut list = f.debug_list();
                list.entry(&"(");
                list.entries(Iterator::intersperse(tys.iter().map(|x| x as &dyn Debug), &","));
                if tys.len() == 1 {
                    list.entry(&",");
                }
                list.entry(&")");
                list.finish()
            }

            PathSegment::FnPtr { tys, output } => {
                let mut list = f.debug_list();
                list.entry(&"fn");
                list.entry(&"(");
                list.entries(Iterator::intersperse(tys.iter().map(|x| x as &dyn Debug), &","));
                list.entry(&")");
                if let Some(output) = output {
                    list.entry(&"->");
                    list.entry(output);
                }
                list.finish()
            }
            PathSegment::Array { ty, length, length_ty } => {
                let mut list = f.debug_list();
                list.entry(&"[");
                list.entry(ty);
                list.entry(&";");
                list.entry(length);
                list.entry(&":");
                list.entry(length_ty);
                list.entry(&"]");
                list.finish()
            }
        }
    }
}
