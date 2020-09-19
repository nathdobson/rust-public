use crate::atomic::{Atomic, usize2, AtomicPackable};
use std::sync::atomic::AtomicU128;
use std::task::{Waker, Poll, Context, RawWaker, RawWakerVTable};
use std::{mem, fmt};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering::AcqRel;
use std::mem::ManuallyDrop;
use crate::waker::AtomicWaker::{Waking, Cancelled};
use std::fmt::{Debug, Formatter};

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord)]
pub struct WakerValue(u128);

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Debug)]
pub enum AtomicWaker {
    Cancelled,
    Waking,
    Waiting(WakerValue),
}

impl WakerValue {
    pub unsafe fn encode(waker: Waker) -> Self {
        WakerValue(mem::transmute(waker))
    }
    pub unsafe fn decode(x: Self) -> Waker {
        mem::transmute(x.0)
    }
}

impl Debug for WakerValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", unsafe { mem::transmute::<_, (*const (), &'static RawWakerVTable)>(self.0) }.0)
    }
}

impl AtomicPackable for AtomicWaker {
    type Raw = usize2;
    unsafe fn decode(x: u128) -> Self {
        match x {
            0 => Cancelled,
            1 => Waking,
            _ => AtomicWaker::Waiting(WakerValue(x)),
        }
    }
    unsafe fn encode(x: Self) -> u128 {
        match x {
            Cancelled => 0,
            Waking => 1,
            AtomicWaker::Waiting(waker) => waker.0,
        }
    }
}

// pub struct AtomicWaker(Atomic<WakerPacker>);
//
// impl AtomicWaker {
//     pub fn new(waker: Waker) -> Self {
//         AtomicWaker(Atomic::new(Some(waker)))
//     }
//     // pub fn new() -> impl Future<Output=AtomicWaker> {
//     //     struct New;
//     //     impl Future for New {
//     //         type Output = AtomicWaker;
//     //         fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//     //             Poll::Ready(AtomicWaker(Atomic::new(Some(cx.waker().clone()))))
//     //         }
//     //     }
//     //     New
//     // }
//     // pub fn wait<'b>(&'b self, on_cancel: impl FnOnce(bool) + 'b) -> impl Future + 'b {
//     //     struct Wait<'a, F: FnOnce(bool)> {
//     //         waker: &'a AtomicWaker,
//     //         entered: bool,
//     //         on_cancel: Option<F>,
//     //     }
//     //     impl<'a, F: FnOnce(bool)> Future for Wait<'a, F> {
//     //         type Output = ();
//     //         fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//     //             unsafe {
//     //                 let this = self.get_unchecked_mut();
//     //                 if !this.entered {
//     //                     this.entered = true;
//     //                     return Poll::Pending;
//     //                 }
//     //                 let old = this.waker.0.swap(Some(cx.waker().clone()), AcqRel);
//     //                 if old.is_none() {
//     //                     this.on_cancel = None;
//     //                     Poll::Ready(())
//     //                 } else {
//     //                     Poll::Pending
//     //                 }
//     //             }
//     //         }
//     //     }
//     //     impl<'a, F: FnOnce(bool)> Drop for Wait<'a, F> {
//     //         fn drop(&mut self) {
//     //             if let Some(on_cancel) = self.on_cancel.take() {
//     //                 let old = self.waker.0.swap(None, AcqRel);
//     //                 on_cancel(old.is_none());
//     //             }
//     //         }
//     //     }
//     //     Wait {
//     //         waker: self,
//     //         entered: false,
//     //         on_cancel: Some(on_cancel),
//     //     }
//     // }
//
//     // #[must_use]
//     // pub fn on_wake(&self, cx: &Context) -> bool {
//     //     self.0.swap(Some(cx.waker().clone()), AcqRel).is_none()
//     // }
//     //
//     // #[must_use]
//     // pub fn try_cancel(&self) -> bool {
//     //     let old = self.waker.0.swap(None, AcqRel);
//     //     on_cancel(old.is_none());
//     // }
//     //
//     // #[must_use]
//     // pub fn complete(&self) -> bool {
//     //     if let Some(waker) = self.0.swap(None, AcqRel) {
//     //         waker.wake();
//     //         true
//     //     } else {
//     //         false
//     //     }
//     // }
// }
//
