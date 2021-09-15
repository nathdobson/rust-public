#![feature(never_type)]
#![feature(const_fn_trait_bound)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_mut_refs)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![deny(unused_must_use)]
#![feature(specialization)]
#![feature(default_free_fn)]
#![allow(incomplete_features)]
#![allow(unreachable_code)]
#![feature(trivial_bounds)]
#![feature(log_syntax)]
#![feature(generic_associated_types)]

use std::marker::PhantomData;
use std::any::{TypeId, Any};
use std::collections::HashMap;
use sha2::Sha256;
use registry::registry;

#[macro_use]
pub mod macros;

pub mod reexport;

mod ser;
mod de;
mod flat_util;

pub mod test_generated { include!(concat!(env!("OUT_DIR"), "/test_generated.rs")); }

pub mod any_generated;
pub mod tag;
#[cfg(test)]
mod test;
pub mod vec_slice;
pub mod buffer;

registry! {
    require tag;
    require any_generated;
}
