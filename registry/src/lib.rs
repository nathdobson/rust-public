#![feature(once_cell, const_fn_fn_ptr_basics)]
//! An alternative to [ctor](https://crates.io/crates/ctor) and [inventory](https://crates.io/crates/inventory) that supports WASM.
//! ```
//! use registry::registry;
//! use registry::Registry;
//!
//! # mod hax {
//! pub mod interface_crate {
//! # use registry::Registry;
//!     # use registry::registry;
//!     # use std::collections::HashSet;
//!     // Define a point where impls can be collected.
//!     pub static IMPL_REGISTRY : Registry<&'static str, HashSet<&'static str>>
//!          = Registry::new(|entries| entries.into_iter().collect());
//!     // The original crate can add impls.
//!     registry! { register(IMPL_REGISTRY) { "native" } }
//! }
//!
//! pub mod impl_crate {
//!     # use registry::registry;
//!     # use super::interface_crate;
//!     // External crates and modules can add impls.
//!     registry! { register(interface_crate::IMPL_REGISTRY) { "external" } }
//! }
//! # } //hax
//! # use hax::interface_crate;
//! # use hax::impl_crate;
//!
//! use interface_crate::IMPL_REGISTRY;
//! registry! {
//!     // Downstream crates must declare dependencies that contain impls
//!     require impl_crate;
//!     require interface_crate;
//!     // Downstream crates can add impls.
//!     register(interface_crate::IMPL_REGISTRY) { "internal" }
//! }
//! // Call once at the beginning of unit tests and main()
//! REGISTRY.build();
//! assert_eq!(*IMPL_REGISTRY, vec!["native", "external", "internal"].into_iter().collect());
//! ```
use parking_lot::Mutex;
use std::lazy::SyncOnceCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::ops::Deref;
use rand::thread_rng;
use rand::seq::SliceRandom;
use std::sync::Once;

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

pub struct Registry<I, O> {
    collect: fn(Vec<I>) -> O,
    inputs: Mutex<Option<Vec<fn() -> I>>>,
    output: SyncOnceCell<O>,
}

impl<I, O> Registry<I, O> {
    pub const fn new(collect: fn(Vec<I>) -> O) -> Self {
        Registry {
            collect,
            inputs: Mutex::new(Some(vec![])),
            output: SyncOnceCell::new(),
        }
    }
    #[doc(hidden)]
    pub fn register(&self, entry: fn() -> I) {
        self.inputs.lock().as_mut().expect("Registry already initialized").push(entry);
    }
}

impl<I, O> Deref for Registry<I, O> {
    type Target = O;

    fn deref(&self) -> &Self::Target {
        self.output.get_or_init(|| {
            let mut vec = self.inputs.lock().take().unwrap();
            vec.shuffle(&mut thread_rng());
            (self.collect)(vec.into_iter().map(|x| x()).collect())
        })
    }
}

#[doc(hidden)]
pub static BUILT_GLOBAL_REGISTRY: AtomicBool = AtomicBool::new(false);

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
    pub fn build(&self) {
        assert!(!BUILT_GLOBAL_REGISTRY.swap(true, Ordering::AcqRel));
        self.build_impl();
        if_inventory! {
            for x in inventory::iter::<&RegistryModule>(){
                assert!(x.state.is_completed(), "Registry not built for {}", x.name);
            }
        }
    }
}

if_inventory! {
    inventory::collect!(&'static RegistryModule);
}

#[macro_export]
macro_rules! registry {
    {
        $(require $mod:tt;)* $(register ($name:expr) { $expr:expr })*
    } => {
        pub static REGISTRY: $crate::RegistryModule = $crate::RegistryModule::new(
            module_path!(),
            ||{
                $(
                    $mod::REGISTRY.build_impl();
                )*
                $(
                    $name.register(|| $expr)
                )*
            }
        );
        $crate::if_inventory! {
            inventory::submit!(&REGISTRY);
        }
    }
}
