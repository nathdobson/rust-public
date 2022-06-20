#![feature(default_free_fn)]
#![allow(unused_imports)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
#![feature(const_transmute_copy)]
#![feature(never_type)]
#![feature(type_alias_impl_trait)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![allow(dead_code)]

use std::any::{Any, TypeId};
use std::borrow::Borrow;
use std::cell::{Cell, UnsafeCell};
use std::collections::HashMap;
use std::default::default;
use std::hash::Hash;
use std::mem::{size_of, ManuallyDrop, MaybeUninit};
use std::ops::Deref;
use std::panic::resume_unwind;
use std::sync::atomic::Ordering::Release;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::thread::ThreadId;
use std::time::Duration;
use std::{mem, thread};

use cache_map::CacheMap;
use ondrop::OnDrop;
use parking_lot::{Mutex, ReentrantMutex};

mod map;
mod safe_lazy;
mod safe_once_cell;

pub use cov_lazy::*;
pub use map::{SafeOnceCellMap, SafeTypeMap};
pub use safe_lazy::SafeLazy;
pub use safe_once_cell::{SafeOnceCell, SafeOnceGuard};

mod cov_cell;
mod cov_lazy;
#[cfg(test)]
mod tests;
