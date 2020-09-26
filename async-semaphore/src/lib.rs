#![allow(unused_imports, incomplete_features)]
#![feature(integer_atomics)]
#![feature(is_sorted)]
#![feature(test)]
#![feature(result_contains_err)]
#![feature(wake_trait)]
#![feature(cfg_target_has_atomic)]
#![feature(const_fn, raw)]
#![feature(unboxed_closures)]
#![feature(unsize)]
#![feature(const_generics)]
#![feature(const_raw_ptr_deref)]
extern crate test;

mod util;
mod atomic;
pub mod shared_dwcas;
pub mod shared_swcas;
pub mod shared_mutex;
//pub mod local;
//pub mod shared;
mod freelist;
mod queue;
// #[cfg(test)]
// mod bench;
//pub mod local;
//#[cfg(test)]
//mod profile;
#[cfg(test)]
mod tests;

use std::fmt::Display;
use std::error::Error;
use std::future::Future;
use std::task::{Waker, Poll, Context};
use std::pin::Pin;
use std::{mem, thread, ptr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::cell::UnsafeCell;
use std::marker::{PhantomData, Unsize};
use std::ops::Deref;
use std::borrow::Borrow;
use std::mem::{size_of, MaybeUninit};
use std::raw::TraitObject;
use crate::util::force_transmute;
use std::any::Any;
use std::ptr::{null, null_mut};

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct Disconnected;

impl Error for Disconnected {}

impl Display for Disconnected {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub struct WouldBlock;

impl Error for WouldBlock {}

impl Display for WouldBlock {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
pub enum TryAcquireError {
    WouldBlock,
    Disconnected,
}

impl Error for TryAcquireError {}

impl Display for TryAcquireError {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// pub trait ArcFuture {
//     type Output;
//     type Target;
//     fn poll(self: Pin<&mut Self>, borrow: &Arc<Self::Target>, cx: &mut Context) -> Poll::<Self::Output>;
// }
//
// struct WeakFuture<'a, T: ArcFuture> {
//     borrow: &'a Weak<T::Target>,
//     fut: T,
// }
//
// impl<'a, T: ArcFuture> Future for WeakFuture<'a, T> {
//     type Output = Result<T::Output, Disconnected>;
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         unsafe {
//             match self.borrow.upgrade() {
//                 None => Poll::Ready(Err(Disconnected)),
//                 Some(borrow) =>
//                     match Pin::new_unchecked(&mut self.fut).poll(&borrow, cx) {
//                         Poll::Ready(x) => Poll::Ready(Ok(x)),
//                         Poll::Pending => Poll::Pending,
//                     },
//             }
//         }
//     }
// }
//
// struct StrongFuture<'a, T: ArcFuture> {
//     borrow: &'a Arc<T::Target>,
//     fut: T,
// }
//
// impl<'a, T: ArcFuture> Future for StrongFuture<'a, T> {
//     type Output = T::Output;
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         unsafe {
//             Pin::new_unchecked(&mut self.fut).poll(self.borrow, cx)
//         }
//     }
// }

pub trait AcquireRelease: Sized {
    #[must_use]
    type Acq;
    fn new(initial: usize) -> Self;
    unsafe fn acquire_new(&self, amount: usize) -> Self::Acq;
    unsafe fn acquire_poll(&self, acq: Pin<&mut Self::Acq>, cx: &mut Context) -> Poll<()>;
    unsafe fn acquire_drop(&self, acq: Pin<&mut Self::Acq>);
    fn try_acquire(&self, amount: usize) -> Result<(), WouldBlock>;
    fn release(&self, amount: usize);
}

pub struct Semaphore<T: AcquireRelease>(Arc<T>);

pub struct WeakSemaphore<T: AcquireRelease>(Weak<T>);

impl<T: AcquireRelease> Semaphore<T> {
    pub fn new(initial: usize) -> Self {
        Semaphore(Arc::new(T::new(initial)))
    }
    pub fn acquire(&self, amount: usize) -> impl Future<Output=ReleaseGuard<T>> + '_ {
        unsafe {
            struct Impl<'a, T2: AcquireRelease> {
                semaphore: &'a Semaphore<T2>,
                amount: usize,
                acq: T2::Acq,
            }
            impl<'a, T2: AcquireRelease> Future for Impl<'a, T2> {
                type Output = ReleaseGuard<T2>;
                fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                    unsafe {
                        let this = self.get_unchecked_mut();
                        match this.semaphore.0.acquire_poll(Pin::new_unchecked(&mut this.acq), cx) {
                            Poll::Ready(()) =>
                                Poll::Ready(ReleaseGuard::new(this.semaphore.0.clone(), this.amount)),
                            Poll::Pending => Poll::Pending
                        }
                    }
                }
            }
            impl<'a, T2: AcquireRelease> Drop for Impl<'a, T2> {
                fn drop(&mut self) {
                    unsafe {
                        self.semaphore.0.acquire_drop(Pin::new_unchecked(&mut self.acq));
                    }
                }
            }
            Impl { semaphore: &*self, acq: self.0.acquire_new(amount), amount }
        }
    }
    pub fn try_acquire(&self, amount: usize) -> Result<ReleaseGuard<T>, WouldBlock> {
        self.0.try_acquire(amount)?;
        Ok(ReleaseGuard::new(self.0.clone(), amount))
    }
    pub fn release(&self, amount: usize) {
        T::release(&self.0, amount)
    }
    pub fn downgrade(&self) -> WeakSemaphore<T> {
        WeakSemaphore(Arc::downgrade(&self.0))
    }
}

impl<T: AcquireRelease> WeakSemaphore<T> {
    pub fn acquire(&self, amount: usize) -> impl Future<Output=Result<ReleaseGuard<T>, Disconnected>> + '_ {
        struct Impl<'a, T2: AcquireRelease> {
            semaphore: &'a WeakSemaphore<T2>,
            amount: usize,
            acq: Option<T2::Acq>,
        }
        impl<'a, T2: AcquireRelease> Future for Impl<'a, T2> {
            type Output = Result<ReleaseGuard<T2>, Disconnected>;
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                unsafe {
                    let this = self.get_unchecked_mut();
                    let semaphore = this.semaphore.0.upgrade().ok_or(Disconnected)?;

                    if this.acq.is_none() {
                        this.acq = Some(semaphore.acquire_new(this.amount));
                    }
                    match semaphore.acquire_poll(Pin::new_unchecked(this.acq.as_mut().unwrap()), cx) {
                        Poll::Ready(()) =>
                            Poll::Ready(Ok(ReleaseGuard::new(semaphore, this.amount))),
                        Poll::Pending => Poll::Pending
                    }
                }
            }
        }
        impl<'a, T2: AcquireRelease> Drop for Impl<'a, T2> {
            fn drop(&mut self) {
                unsafe {
                    if let Some(semaphore) = self.semaphore.0.upgrade() {
                        if let Some(acq) = self.acq.as_mut() {
                            semaphore.acquire_drop(Pin::new_unchecked(acq));
                        }
                    }
                }
            }
        }
        Impl { semaphore: &*self, acq: None, amount }
    }
    pub fn try_acquire(&self, amount: usize) -> Result<ReleaseGuard<T>, TryAcquireError> {
        let this = self.0.upgrade().ok_or(TryAcquireError::Disconnected)?;
        this.try_acquire(amount).map_err(|WouldBlock| TryAcquireError::WouldBlock)?;
        Ok(ReleaseGuard::new(this, amount))
    }
}

impl<T: AcquireRelease> Clone for Semaphore<T> {
    fn clone(&self) -> Self {
        Semaphore(self.0.clone())
    }
}

impl<T: AcquireRelease> Clone for WeakSemaphore<T> {
    fn clone(&self) -> Self {
        WeakSemaphore(self.0.clone())
    }
}

// 
// pub trait AcquireExt {
//     type Inner: AcquireRelease;
//     type Acq: Future<Output=Result<ReleaseGuard<Self::Inner>, Disconnected>>;
//     fn acquire(&self, amount: usize) -> Self::Acq;
//     fn try_acquire(&self, amount: usize) -> Result<ReleaseGuard<Self::Inner>, TryAcquireError>;
// }
// 
// impl<T: AcquireRelease> AcquireExt for Weak<T> {
//     type Inner = T;
//     type Acq = T::Acq;
// 
//     fn acquire(&self, amount: usize) -> Self::Acq {
//         Self::acquire(self, amount)
//     }
// 
//     fn try_acquire(&self, amount: usize) -> Result<ReleaseGuard<Self::Inner>, TryAcquireError> {
//         match self.upgrade() {
//             None => Err(TryAcquireError::Disconnected),
//             Some(this) => T::try_acquire(this, amount)
//         }
//     }
// }

pub struct ReleaseGuard<R> where R: AcquireRelease {
    releaser: Option<Arc<R>>,
    amount: usize,
}

impl<R: AcquireRelease> ReleaseGuard<R> where R: AcquireRelease {
    pub fn new(releaser: Arc<R>, amount: usize) -> Self {
        ReleaseGuard { releaser: Some(releaser), amount }
    }
    pub fn forget(mut self) {
        self.releaser = None;
        mem::forget(self);
    }
}

impl<R> Drop for ReleaseGuard<R> where R: AcquireRelease {
    fn drop(&mut self) {
        if let Some(releaser) = self.releaser.borrow() {
            R::release(releaser, self.amount);
        }
    }
}
