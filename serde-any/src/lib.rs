#![feature(specialization, never_type, const_fn_fn_ptr_basics)]
#![allow(incomplete_features, unused_variables, dead_code, unused_imports, unused_macros, unused_mut)]
#![deny(unused_must_use)]

#[macro_use]
mod macros;
mod tag;
#[cfg(test)]
mod test;
mod binary;
mod ser;
mod any;
mod de;

