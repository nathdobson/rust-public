#![feature(specialization, never_type, const_fn_fn_ptr_basics)]
#![allow(incomplete_features, unused_variables, dead_code, unused_imports, unused_macros, unused_mut)]
#![deny(unused_must_use)]

#[macro_use]
mod macros;
pub mod tag;
#[cfg(test)]
mod test;
pub mod binary;
pub mod ser;
pub mod any;
mod de;

