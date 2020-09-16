use std::sync::atomic::{Ordering,
                        AtomicU128, AtomicU64, AtomicU32, AtomicU16, AtomicU8, AtomicUsize};
use std::marker::PhantomData;
use std::mem;
use std::mem::size_of;
use std::sync::Mutex;
use std::ops::{DerefMut, Deref};
use std::cell::{Cell, RefCell, Ref, RefMut};

pub struct Atomic<T: AtomicPacker>(T::Impl);

pub unsafe trait AtomicInteger {
    type Raw;
    fn new(val: Self::Raw) -> Self;
    fn load(&self, ordering: Ordering) -> Self::Raw;
    fn store(&self, new: Self::Raw, ordering: Ordering);
    fn compare_exchange_weak(&self,
                             current: Self::Raw,
                             new: Self::Raw,
                             success: Ordering,
                             failure: Ordering) -> Result<Self::Raw, Self::Raw>;
    fn swap(&self, new: Self::Raw, ordering: Ordering) -> Self::Raw;
    fn fetch_update(&self, set_order: Ordering, fetch_order: Ordering, f: impl FnMut(Self::Raw)
        -> Option<Self::Raw>) -> Result<Self::Raw, Self::Raw>;
}

pub trait AtomicPacker {
    type Impl: AtomicInteger;
    type Value;
    unsafe fn encode(val: Self::Value) -> <Self::Impl as AtomicInteger>::Raw;
    unsafe fn decode(val: <Self::Impl as AtomicInteger>::Raw) -> Self::Value;
}


impl<T: AtomicPacker> Atomic<T> {
    pub fn new(val: T::Value) -> Self {
        unsafe {
            Atomic(T::Impl::new(T::encode(val)))
        }
    }
    pub fn load(&self, ordering: Ordering) -> T::Value where T::Value: Copy {
        unsafe { T::decode(self.0.load(ordering)) }
    }
    pub fn store(&self, new: T::Value, ordering: Ordering) where T::Value: Copy {
        unsafe { self.0.store(T::encode(new), ordering) }
    }

    pub fn compare_exchange_weak(&self,
                                 current: T::Value,
                                 new: T::Value,
                                 success: Ordering,
                                 failure: Ordering) -> Result<T::Value, T::Value> where T::Value: Copy {
        unsafe {
            match self.0.compare_exchange_weak(
                T::encode(current), T::encode(new),
                success, failure) {
                Ok(ok) => Ok(T::decode(ok)),
                Err(err) => Err(T::decode(err)),
            }
        }
    }

    #[must_use]
    pub fn compare_update_weak(&self,
                               current: &mut T::Value,
                               new: T::Value,
                               success: Ordering,
                               failure: Ordering) -> bool where T::Value: Copy {
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

    pub fn swap(&self, new: T::Value, ordering: Ordering) -> T::Value {
        unsafe {
            T::decode(self.0.swap(T::encode(new), ordering))
        }
    }
}

impl<T: AtomicPacker> Drop for Atomic<T> {
    fn drop(&mut self) {
        unsafe { T::decode(self.0.load(Ordering::Relaxed)) };
    }
}

macro_rules! impl_atomic_integer {
    ($atomic:ty, $raw:ty) => {
        unsafe impl AtomicInteger for $atomic {
            type Raw = $raw;

            fn new(val: Self::Raw) -> Self { Self::new(val) }

            fn load(&self, ordering: Ordering) -> Self::Raw { self.load(ordering) }

            fn store(&self, new: Self::Raw, ordering: Ordering) { self.store(new, ordering) }

            fn compare_exchange_weak(&self,
                                     current: Self::Raw,
                                     new: Self::Raw,
                                     success: Ordering,
                                     failure: Ordering)
                                     -> Result<Self::Raw, Self::Raw> {
                self.compare_exchange_weak(current, new, success, failure)
            }
            fn swap(&self, new: Self::Raw, ordering: Ordering) -> Self::Raw {
                self.swap(new, ordering)
            }
            fn fetch_update(&self,
                            set_order: Ordering,
                            fetch_order: Ordering,
                            f: impl FnMut(Self::Raw) -> Option<Self::Raw>)
                            -> Result<Self::Raw, Self::Raw> {
                self.fetch_update(set_order, fetch_order, f)
            }
        }
    }
}
impl_atomic_integer!(AtomicU128, u128);
impl_atomic_integer!(AtomicU64, u64);
impl_atomic_integer!(AtomicU32, u32);
impl_atomic_integer!(AtomicU16, u16);
impl_atomic_integer!(AtomicU8, u8);
impl_atomic_integer!(AtomicUsize, usize);

#[cfg(target_pointer_width = "32")]
pub type AtomicUsize2 = AtomicU64;

#[cfg(target_pointer_width = "64")]
pub type AtomicUsize2 = AtomicU128;

#[allow(non_camel_case_types)]
pub type usize2 = <AtomicUsize2 as AtomicInteger>::Raw;

pub struct CastPacker<V, I: AtomicInteger>(PhantomData<(V, I)>);

unsafe fn force_transmute<T, U>(value: T) -> U {
    assert_eq!(size_of::<T>(), size_of::<U>());
    let result = mem::transmute_copy(&value);
    mem::forget(value);
    result
}

impl<V, I: AtomicInteger> AtomicPacker for CastPacker<V, I> {
    type Impl = I;
    type Value = V;
    unsafe fn encode(val: V) -> I::Raw {
        force_transmute(val)
    }
    unsafe fn decode(val: I::Raw) -> V {
        force_transmute(val)
    }
}
