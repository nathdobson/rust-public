use crate::atomic::{Atomic, usize2, AtomicPackable};
use std::task::{Waker, Poll, Context, RawWakerVTable};
use std::{mem, fmt};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering::AcqRel;
use std::mem::ManuallyDrop;
use std::fmt::{Debug, Formatter};
use crate::shared_dwcas::state::WaiterState::{Cancelled, Waking};

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord)]
pub struct RawWaker(usize2);

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Debug)]
pub enum WaiterState {
    Cancelled,
    Waking,
    Sleeping(RawWaker),
}

impl RawWaker {
    pub unsafe fn encode(waker: Waker) -> Self {
        RawWaker(mem::transmute(waker))
    }
    pub unsafe fn decode(x: Self) -> Waker {
        mem::transmute(x.0)
    }
}

impl Debug for RawWaker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", unsafe { mem::transmute::<_, (*const (), &'static RawWakerVTable)>(self.0) }.0)
    }
}

impl AtomicPackable for WaiterState {
    type Raw = usize2;
    unsafe fn decode(x: usize2) -> Self {
        match x {
            0 => Cancelled,
            1 => Waking,
            _ => WaiterState::Sleeping(RawWaker(x)),
        }
    }
    unsafe fn encode(x: Self) -> usize2 {
        match x {
            Cancelled => 0,
            Waking => 1,
            WaiterState::Sleeping(waker) => waker.0,
        }
    }
}
