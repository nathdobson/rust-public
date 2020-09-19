use std::sync::atomic::{Ordering};
use std::marker::PhantomData;
use std::mem;
use std::mem::size_of;
use std::sync::Mutex;
use std::ops::{DerefMut, Deref};
use std::cell::{Cell, RefCell, Ref, RefMut};

pub struct Atomic<T: AtomicPackable>(<T::Raw as HasAtomic>::Impl) where T::Raw: HasAtomic;

pub unsafe trait HasAtomic: Sized {
    type Impl;
    fn new(val: Self) -> Self::Impl;
    fn load(this: &Self::Impl, ordering: Ordering) -> Self;
    fn store(this: &Self::Impl, new: Self, ordering: Ordering);
    fn compare_exchange_weak(this: &Self::Impl,
                             current: Self,
                             new: Self,
                             success: Ordering,
                             failure: Ordering) -> Result<Self, Self>;
    fn swap(this: &Self::Impl, new: Self, ordering: Ordering) -> Self;
    fn fetch_update(this: &Self::Impl, set_order: Ordering, fetch_order: Ordering, f: impl FnMut(Self)
        -> Option<Self>) -> Result<Self, Self>;
}

unsafe fn force_transmute<T, U>(value: T) -> U {
    assert_eq!(size_of::<T>(), size_of::<U>());
    let result = mem::transmute_copy(&value);
    mem::forget(value);
    result
}

pub trait AtomicPackable: Sized {
    type Raw;
    unsafe fn encode(val: Self) -> Self::Raw {
        force_transmute(val)
    }
    unsafe fn decode(val: Self::Raw) -> Self {
        force_transmute(val)
    }
}

impl<T: AtomicPackable> Atomic<T> where T::Raw: HasAtomic {
    pub fn new(val: T) -> Self {
        unsafe {
            Atomic(T::Raw::new(T::encode(val)))
        }
    }
    pub fn load(&self, ordering: Ordering) -> T where T: Copy {
        unsafe { T::decode(T::Raw::load(&self.0, ordering)) }
    }
    pub fn store(&self, new: T, ordering: Ordering) where T: Copy {
        unsafe { T::Raw::store(&self.0, T::encode(new), ordering) }
    }

    pub fn compare_exchange_weak(&self,
                                 current: T,
                                 new: T,
                                 success: Ordering,
                                 failure: Ordering) -> Result<T, T> where T: Copy {
        unsafe {
            match T::Raw::compare_exchange_weak(
                &self.0,
                T::encode(current), T::encode(new),
                success, failure) {
                Ok(ok) => Ok(T::decode(ok)),
                Err(err) => Err(T::decode(err)),
            }
        }
    }

    #[must_use]
    pub fn compare_update_weak(&self,
                               current: &mut T,
                               new: T,
                               success: Ordering,
                               failure: Ordering) -> bool where T: Copy {
        match self.compare_exchange_weak(*current, new, success, failure) {
            Ok(_) => {
                *current = new;
                true
            }
            Err(actual) => {
                *current = actual;
                false
            }
        }
    }

    pub fn swap(&self, new: T, ordering: Ordering) -> T {
        unsafe {
            T::decode(T::Raw::swap(&self.0, T::encode(new), ordering))
        }
    }
}

impl<T: AtomicPackable> Drop for Atomic<T> where T::Raw: HasAtomic {
    fn drop(&mut self) {
        unsafe { T::decode(T::Raw::load(&self.0, Ordering::Relaxed)) };
    }
}

macro_rules! impl_atomic_integer {
    ($atomic:ty, $raw:ty) => {
        unsafe impl HasAtomic for $raw {
            type Impl = $atomic;

            fn new(val: Self) -> Self::Impl { Self::Impl::new(val) }

            fn load(this: &Self::Impl, ordering: Ordering) -> Self { this.load(ordering) }

            fn store(this: &Self::Impl, new: Self, ordering: Ordering) { this.store(new, ordering) }

            fn compare_exchange_weak(this: &Self::Impl,
                                     current: Self,
                                     new: Self,
                                     success: Ordering,
                                     failure: Ordering)
                                     -> Result<Self, Self> {
                this.compare_exchange_weak(current, new, success, failure)
            }
            fn swap(this: &Self::Impl, new: Self, ordering: Ordering) -> Self {
                this.swap(new, ordering)
            }
            fn fetch_update(this: &Self::Impl,
                            set_order: Ordering,
                            fetch_order: Ordering,
                            f: impl FnMut(Self) -> Option<Self>)
                            -> Result<Self, Self> {
                this.fetch_update(set_order, fetch_order, f)
            }
        }
    }
}
#[cfg(target_has_atomic = "8")]
impl_atomic_integer!(std::sync::atomic::AtomicU8, u8);
#[cfg(target_has_atomic = "16")]
impl_atomic_integer!(std::sync::atomic::AtomicU16, u16);
#[cfg(target_has_atomic = "32")]
impl_atomic_integer!(std::sync::atomic::AtomicU32, u32);
#[cfg(target_has_atomic = "64")]
impl_atomic_integer!(std::sync::atomic::AtomicU64, u64);
#[cfg(target_has_atomic = "128")]
impl_atomic_integer!(std::sync::atomic::AtomicU128, u128);

#[cfg(target_pointer_width = "32")]
#[allow(non_camel_case_types)]
pub type usize1 = u32;
#[cfg(target_pointer_width = "64")]
#[allow(non_camel_case_types)]
pub type usize1 = u64;

#[cfg(target_pointer_width = "32")]
#[allow(non_camel_case_types)]
pub type usize2 = u64;
#[cfg(target_pointer_width = "64")]
#[allow(non_camel_case_types)]
pub type usize2 = u128;

impl<T> AtomicPackable for *const T { type Raw = usize1; }

impl<T> AtomicPackable for *mut T { type Raw = usize1; }

impl AtomicPackable for usize { type Raw = usize1; }

impl AtomicPackable for isize { type Raw = usize1; }

impl AtomicPackable for u8 { type Raw = u8; }

impl AtomicPackable for i8 { type Raw = u8; }

impl AtomicPackable for u16 { type Raw = u16; }

impl AtomicPackable for i16 { type Raw = u16; }

impl AtomicPackable for u32 { type Raw = u32; }

impl AtomicPackable for i32 { type Raw = u32; }

impl AtomicPackable for u64 { type Raw = u64; }

impl AtomicPackable for i64 { type Raw = u64; }

impl AtomicPackable for u128 { type Raw = u128; }

impl AtomicPackable for i128 { type Raw = u128; }

pub trait Double { type Output; }

impl Double for u8 { type Output = u16; }

impl Double for u16 { type Output = u32; }

impl Double for u32 { type Output = u64; }

impl Double for u64 { type Output = u128; }

impl<A, B> AtomicPackable for (A, B)
    where A: AtomicPackable,
          B: AtomicPackable<Raw=A::Raw>,
          A::Raw: Double {
    type Raw = <A::Raw as Double>::Output;
}
