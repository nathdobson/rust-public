use std::sync::atomic::AtomicUsize;
use std::cell::UnsafeCell;
use std::task::{Waker, Poll};
use crate::atomic::{Atomic, AtomicPackable, usize1};
use crate::shared_swcas::waker::State::{Sleeping, Storing, Finished, Cancelled};
use std::sync::atomic::Ordering::{Acquire, AcqRel};

#[derive(Copy, Clone, Eq, PartialOrd, PartialEq, Ord, Debug)]
enum State {
    Sleeping,
    Storing,
    Finished,
    Cancelled,
}

pub struct AtomicWaker {
    state: Atomic<State>,
    waker: UnsafeCell<Option<Waker>>,
}

impl AtomicPackable for State {
    type Raw = u8;
}

#[derive(Eq, Ord, PartialOrd, PartialEq, Debug)]
pub struct PendingError;

#[derive(Eq, Ord, PartialOrd, PartialEq, Debug)]
pub struct CancelledError;

#[derive(Eq, Ord, PartialOrd, PartialEq, Debug)]
pub struct FinishedError;

impl AtomicWaker {
    pub fn new(waker: Waker) -> Self {
        AtomicWaker {
            state: Atomic::new(Sleeping),
            waker: UnsafeCell::new(Some(waker)),
        }
    }
    pub fn poll(&self, waker: &Waker) -> Result<(), PendingError> {
        unsafe {
            match self.state.compare_and_swap(Sleeping, Storing, AcqRel) {
                Sleeping => {
                    *self.waker.get() = Some(waker.clone());
                    match self.state.compare_and_swap(Storing, Sleeping, AcqRel) {
                        Storing => return Err(PendingError),
                        Finished => {
                            (*self.waker.get()).take().unwrap().wake();
                            return Ok(());
                        }
                        Sleeping => unreachable!(),
                        Cancelled => panic!("poll after cancel"),
                    }
                }
                Finished => return Ok(()),
                Storing => panic!("concurrent poll"),
                Cancelled => panic!("poll after cancel"),
            }
        }
    }
    pub fn finish(&self) -> Result<(), CancelledError> {
        unsafe {
            match self.state.swap(Finished, AcqRel) {
                Sleeping => {
                    (*self.waker.get()).take().unwrap().wake();
                    return Ok(());
                }
                Storing => return Ok(()),
                Finished => panic!("Finishing twice"),
                Cancelled => return Err(CancelledError),
            }
        }
    }
    pub fn cancel(&self) -> Result<(), FinishedError> {
        unsafe {
            match self.state.swap(Cancelled, AcqRel) {
                Sleeping => {
                    (*self.waker.get()).take().unwrap();
                    return Ok(());
                }
                Storing => return Ok(()),
                Finished => return Err(FinishedError),
                Cancelled => panic!("Cancelling twice"),
            }
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.state.load(Acquire) == Cancelled
    }
}
