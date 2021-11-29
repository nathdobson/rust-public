#![allow(dead_code, unused_imports)]
#![feature(once_cell, const_fn_fn_ptr_basics, const_mut_refs, const_fn_trait_bound)]
#![feature(thread_id_value)]
#![allow(unused_mut)]
//! An alternative to [ctor](https://crates.io/crates/ctor) and [inventory](https://crates.io/crates/inventory) that supports WASM.
//! ```
//! use registry::registry;
//! use registry::Registry;
//!
//! # mod hax {
//! pub mod interface_crate {
//!     # use registry::{Registry, Builder, BuilderFrom};
//!     # use registry::registry;
//!     # use std::collections::HashSet;
//!     pub struct Impls(HashSet<&'static str>);
//!     impl Builder for Impls {
//!         type Output = HashSet<&'static str>;
//!         fn new() -> Self { Impls(HashSet::new()) }
//!         fn build(self) -> Self::Output { self.0 }
//!     }
//!     impl BuilderFrom<&'static str> for Impls{
//!         fn insert(&mut self, element: &'static str) {
//!             self.0.insert(element);
//!         }
//!     }
//!     // Define a point where impls can be collected.
//!     pub static IMPLS : Registry<Impls> = Registry::new();
//!     // The original crate can add impls.
//!     registry! { value IMPLS => "native"; }
//! }
//!
//! pub mod impl_crate {
//!     # use registry::registry;
//!     # use super::interface_crate;
//!     // External crates and modules can add impls.
//!     registry! { value interface_crate::IMPLS => "external"; }
//! }
//! # } //hax
//! # use hax::interface_crate;
//! # use hax::impl_crate;
//!
//! use interface_crate::IMPLS;
//! registry! {
//!     // Downstream crates must declare dependencies that contain impls
//!     require impl_crate;
//!     require interface_crate;
//!     // Downstream crates can add impls.
//!     value IMPLS => "internal";
//! }
//! // Call once at the beginning of unit tests and main()
//! REGISTRY.build();
//! assert_eq!(*IMPLS, vec!["native", "external", "internal"].into_iter().collect());
//! ```
pub mod safe_cell;

use parking_lot::Mutex;
use std::lazy::{SyncOnceCell, Lazy, SyncLazy};
use std::ops::Deref;
use rand::thread_rng;
use rand::seq::SliceRandom;
use std::sync::Once;
use std::fmt::{Debug, Formatter};
use crate::safe_cell::{SafeLazy, SafeOnceCell};

#[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd", target_os = "illumos", target_os = "macos", target_os = "ios", windows))]
#[macro_export]
#[doc(hidden)]
macro_rules! if_inventory {
    ($($x:tt)*) => {$($x)*}
}

#[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd", target_os = "illumos", target_os = "macos", target_os = "ios", windows)))]
#[macro_export]
#[doc(hidden)]
macro_rules! if_inventory {
    ($($x:tt)*) => {}
}


#[doc(hidden)]
pub mod reexport {
    if_inventory! {
        pub use inventory;
    }
}

pub trait Builder {
    type Output;
    fn new() -> Self;
    fn build(self) -> Self::Output;
}

pub trait BuilderFrom<T> {
    fn insert(&mut self, element: T);
}

pub struct Registry<B: Builder> {
    inputs: Mutex<Option<Vec<fn(&mut B)>>>,
    output: SafeOnceCell<B::Output>,
}

pub struct LazyEntry<T: 'static> {
    private: SafeLazy<T>,
    public: SafeLazy<&'static T>,
}

impl<T> LazyEntry<T> {
    #[doc(hidden)]
    pub const fn new(private: fn() -> T, public: fn() -> &'static T) -> Self {
        LazyEntry {
            private: SafeLazy::new(private),
            public: SafeLazy::new(public),
        }
    }
    #[doc(hidden)]
    pub fn private(&self) -> &T { &*self.private }
}

impl<B: Builder> Registry<B> {
    pub const fn new() -> Self {
        Registry {
            inputs: Mutex::new(Some(vec![])),
            output: SafeOnceCell::new(),
        }
    }
    #[doc(hidden)]
    pub fn register(&self, entry: fn(&mut B)) {
        self.inputs.lock().as_mut()
            .expect("Registry already initialized")
            .push(entry);
    }
}

impl<B: Builder> Deref for Registry<B> {
    type Target = B::Output;

    fn deref(&self) -> &Self::Target {
        self.output.get_or_init(|| {
            assert!(BUILT_GLOBAL_REGISTRY.get().is_some(), "REGISTRY.build() must be called");
            let mut vec = self.inputs.lock().take().unwrap();
            vec.shuffle(&mut thread_rng());
            let mut result = B::new();
            for x in vec {
                x(&mut result);
            }
            result.build()
        })
    }
}

impl<T> Deref for LazyEntry<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.public.deref()
    }
}

#[doc(hidden)]
pub static BUILT_GLOBAL_REGISTRY: SyncOnceCell<&'static RegistryModule> = SyncOnceCell::new();

pub struct RegistryModule {
    name: &'static str,
    state: Once,
    init: fn(),
}

impl RegistryModule {
    #[doc(hidden)]
    pub const fn new(name: &'static str, init: fn()) -> Self {
        RegistryModule {
            name,
            state: Once::new(),
            init,
        }
    }
    #[doc(hidden)]
    pub fn build_impl(&self) {
        self.state.call_once(self.init);
    }
    pub fn build(&'static self) {
        let inited = *BUILT_GLOBAL_REGISTRY.get_or_init(|| {
            self.build_impl();
            if_inventory! {
                for x in inventory::iter::<&RegistryModule>(){
                    assert!(x.state.is_completed(), "Registry not built for {}", x.name);
                }
            }
            self
        });
        assert_eq!(inited as *const RegistryModule, self as *const RegistryModule);
    }
}

if_inventory! {
    inventory::collect!(&'static RegistryModule);
}

#[macro_export]
macro_rules! registry {
    {
        $(require $mod:tt;)*
        $(value $table:expr => $expr:expr;)*
        $(static $static_table:expr => $static_vis:vis $static_name:ident: $static_type:ident = $static_exp:expr;)*
        $(lazy $lazy_table:expr => $lazy_vis:vis $lazy_name:ident: $lazy_type:ty = $lazy_exp:expr;)*
        $(type $type_table:expr => $type_name:ty;)*
    } => {
        $(
            $static_vis static $static_name: $static_type = $static_exp;
        )*
        $(
            $lazy_vis static $lazy_name: $crate::LazyEntry<$lazy_type> =
                $crate::LazyEntry::new(|| $lazy_exp, || {
                    ::std::mem::drop(::std::ops::Deref::deref(&$lazy_table));
                    $lazy_name.private()});
        )*
        pub static REGISTRY: $crate::RegistryModule = $crate::RegistryModule::new(
            module_path!(),
            ||{
                $(
                    $mod::REGISTRY.build_impl();
                )*
                $(
                    ($table).register(|b| $crate::BuilderFrom::insert(b, $expr));
                )*
                $(
                    ($static_table).register(|b| $crate::BuilderFrom::insert(b, &$static_name));
                )*
                $(
                    ($lazy_table).register(|b| $crate::BuilderFrom::insert(b, $lazy_name.private()));
                )*
                $(
                    ($type_table).register(|b| $crate::BuilderFrom::insert(b, &::std::marker::PhantomData::<$type_name>));
                )*
                {
                    $crate::if_inventory! {
                        use $crate::reexport::inventory;
                        inventory::submit!(&REGISTRY);
                    }
                }
            }
        );
    }
}

impl<B: Builder> Debug for Registry<B> where B::Output: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("output", self.deref())
            .finish_non_exhaustive()
    }
}
