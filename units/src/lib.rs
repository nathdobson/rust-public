#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![deny(unused_must_use)]
#![feature(trait_alias)]
#![feature(never_type)]
#![feature(unboxed_closures)]
#![feature(box_syntax)]
#![feature(bindings_after_at)]
#![feature(associated_type_bounds)]
#![feature(min_type_alias_impl_trait)]

mod ast;
mod check;
mod eval;
mod factors;
mod map;
mod tests;
mod unicode;
mod unit;
mod value;
mod variants;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(pub grammar); // synthesized by LALRPOP
