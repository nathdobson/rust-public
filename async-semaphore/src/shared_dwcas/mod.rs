mod state;

use std::sync::atomic::{Ordering};
use std::{mem, fmt};
use std::marker::PhantomData;
use std::ptr::null;
use std::mem::{MaybeUninit, size_of};
use std::cell::UnsafeCell;
use std::task::{Waker, Context, Poll};
use std::sync::{Mutex, Weak, Arc};
use std::future::Future;
use std::pin::Pin;
use std::ops::DerefMut;
use std::fmt::{Debug, Formatter};
use crate::atomic::{Atomic, usize2, AtomicPackable};
use std::sync::atomic::Ordering::{SeqCst, Relaxed, AcqRel, Acquire, Release};
use crate::shared_dwcas::SemaphoreMode::{Available, Queued, Locked, LockedDirty};
use crate::freelist::FreeList;
use crate::shared_dwcas::state::{WaiterState, RawWaker};
use crate::shared_dwcas::state::WaiterState::Sleeping;
use crate::shared_dwcas::state::WaiterState::Cancelled;
use crate::shared_dwcas::state::WaiterState::Waking;
use crate::{ReleaseGuard, TryAcquireError, AcquireRelease, Disconnected};
use crate::WouldBlock;

pub struct SemaphoreImpl {
    state: Atomic<SemaphoreState>,
    front: UnsafeCell<*const Waiter>,
    freelist: FreeList<Waiter>,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
enum SemaphoreMode {
    Available,
    Queued,
    Locked,
    LockedDirty,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct SemaphoreState {
    available: usize,
    back: *const Waiter,
    mode: SemaphoreMode,
}

#[repr(align(64))]
pub struct Waiter {
    waker: Atomic<WaiterState>,
    next: UnsafeCell<*const Waiter>,
    amount: usize,
}

pub enum AcquireStep {
    Enter,
    Loop(*const Waiter),
    Poison,
}

pub struct AcquireImpl {
    amount: usize,
    step: AcquireStep,
}

impl Unpin for AcquireImpl{}

impl AtomicPackable for SemaphoreState {
    type Raw = usize2;
    unsafe fn encode(state: SemaphoreState) -> usize2 {
        ((state.available as usize2) << (size_of::<usize>() * 8)) |
            (state.back as usize2) |
            (match state.mode {
                Available => 0,
                Queued => 1,
                Locked => 2,
                LockedDirty => 3,
            })
    }

    unsafe fn decode(x: usize2) -> SemaphoreState {
        SemaphoreState {
            available: (x >> (size_of::<usize>() * 8)) as usize,
            back: ((x as usize) & (!3usize)) as *const Waiter,
            mode: match x & 3 {
                0 => Available,
                1 => Queued,
                2 => Locked,
                3 => LockedDirty,
                _ => unreachable!(),
            },
        }
    }
}

unsafe impl Sync for SemaphoreImpl {}

unsafe impl Send for SemaphoreImpl {}

unsafe impl Sync for AcquireImpl {}

unsafe impl Send for AcquireImpl {}

struct DebugPtr(*const Waiter);

impl Debug for DebugPtr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            write!(f, "{:?}", self.0)?;
            if self.0 != null() {
                write!(f, " {:?} {:?} {:?}", (*self.0).amount, (*self.0).waker.load(Ordering::Relaxed), DebugPtr(*(*self.0).next.get()))?;
            }
            Ok(())
        }
    }
}

impl Drop for SemaphoreImpl {
    fn drop(&mut self) {
        unsafe {
            for &(mut ptr) in &[*self.front.get(),
                self.state.load(Relaxed).back] {
                while ptr != null() {
                    let next = *(*ptr).next.get();
                    self.freelist.free(ptr);
                    ptr = next;
                }
            }
        }
    }
}

impl Debug for SemaphoreImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unsafe {
            let mut w = f.debug_struct("Semaphore");
            let state = self.state.load(SeqCst);
            w.field("A", &state.available);
            w.field("M", &state.mode);
            w.field("B", &DebugPtr(state.back));
            w.field("F", &DebugPtr(*self.front.get()));
            w.finish()
        }
    }
}

impl SemaphoreImpl {
    pub fn available(&self) -> usize {
        self.state.load(SeqCst).available
    }

    unsafe fn try_acquire_or_push(&self, amount: usize, waiter: *const Waiter) -> bool {
        let mut old_state = self.state.load(Acquire);
        loop {
            let mut state = old_state;
            if amount <= state.available && state.mode == Available {
                state.available -= amount;
                if self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    match (*waiter).waker.swap(Cancelled, Relaxed) {
                        Sleeping(raw_waker) => mem::drop(RawWaker::decode(raw_waker)),
                        _ => unreachable!(),
                    }
                    self.freelist.free(waiter);
                    return true;
                }
            } else {
                *(*waiter).next.get() = state.back;
                if state.mode == Available {
                    state.mode = Queued;
                }
                state.back = waiter;
                if self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    return false;
                }
            }
        }
    }

    unsafe fn unlock(&self) {
        let mut old_state = self.state.load(Acquire);
        loop {
            let mut state = old_state;
            if state.mode == LockedDirty {
                state.mode = Locked;
                if !self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    continue;
                }
            }

            if *self.front.get() == null() {
                if state.back == null() {
                    state.mode = Available;
                    if !self.state.compare_update_weak(
                        &mut old_state, state, AcqRel, Acquire) {
                        continue;
                    }
                    break;
                }
                state.back = null();
                let mut back = old_state.back;
                if !self.state.compare_update_weak(
                    &mut old_state, state, AcqRel, Acquire) {
                    continue;
                }
                while back != null() {
                    let next = *((*back).next.get());
                    *(*back).next.get() = *self.front.get();
                    *self.front.get() = back;
                    back = next;
                }
                assert_ne!((*self.front.get()), null());
                continue;
            }
            let front = *self.front.get();
            if (*front).amount <= state.available {
                *self.front.get() = *(*front).next.get();
                match (*front).waker.swap(WaiterState::Waking, Ordering::AcqRel) {
                    Cancelled => {
                        self.freelist.free(front);
                        continue;
                    }
                    Sleeping(waker) => {
                        loop {
                            state = old_state;
                            state.available -= (*front).amount;
                            if self.state.compare_update_weak(
                                &mut old_state, state, AcqRel, Acquire) {
                                break;
                            }
                        }
                        RawWaker::decode(waker).wake();
                        continue;
                    }
                    Waking => unreachable!()
                }
            }
            match (*front).waker.load(Ordering::Acquire) {
                WaiterState::Cancelled => {
                    *self.front.get() = *(*front).next.get();
                    self.freelist.free(front);
                    continue;
                }
                WaiterState::Sleeping(_waker) => {}
                WaiterState::Waking => unreachable!(),
            }
            state.mode = Queued;
            if self.state.compare_update_weak(
                &mut old_state, state, AcqRel, Acquire) {
                break;
            }
        }
    }
}

impl Drop for Waiter {
    fn drop(&mut self) {
        match self.waker.load(Ordering::Relaxed) {
            Sleeping(w) => panic!("Leaking waker {:?}", w),
            _ => {}
        }
    }
}

impl AcquireRelease for SemaphoreImpl {
    type Acq = AcquireImpl;

    fn new(initial: usize) -> Self {
        SemaphoreImpl {
            state: Atomic::new(SemaphoreState {
                available: initial,
                back: null(),
                mode: Available,
            }),
            front: UnsafeCell::new(null()),
            freelist: FreeList::new(),
        }
    }

    fn release(&self, amount: usize) {
        unsafe {
            let mut old_state = self.state.load(Acquire);
            loop {
                let mut state = old_state;
                state.available += amount;
                match state.mode {
                    Locked | LockedDirty => {
                        if self.state.compare_update_weak(
                            &mut old_state, state, AcqRel, Acquire) {
                            return;
                        }
                    }
                    Queued => {
                        state.mode = Locked;
                        if self.state.compare_update_weak(
                            &mut old_state, state, AcqRel, Acquire) {
                            self.unlock();
                            return;
                        }
                    }
                    Available => {
                        assert_eq!(state.back, null());
                        if self.state.compare_update_weak(
                            &mut old_state, state, AcqRel, Acquire) {
                            return;
                        }
                    }
                }
            }
        }
    }

    unsafe fn acquire_new(&self, amount: usize) -> Self::Acq {
        AcquireImpl {
            amount,
            step: AcquireStep::Enter,
        }
    }

    unsafe fn acquire_poll(&self, mut acq: Pin<&mut AcquireImpl>, cx: &mut Context<'_>) -> Poll<()> {
        match acq.step {
            AcquireStep::Enter => {
                if let Ok(guard) =
                SemaphoreImpl::try_acquire(self, acq.amount) {
                    acq.step = AcquireStep::Poison;
                    return Poll::Ready(guard);
                }
                let waiter =
                    self.freelist.allocate(Waiter {
                        waker: Atomic::new(WaiterState::Sleeping(RawWaker::encode(cx.waker().clone()))),
                        next: UnsafeCell::new(null()),
                        amount: acq.amount,
                    });
                if self.try_acquire_or_push(acq.amount, waiter) {
                    acq.step = AcquireStep::Poison;
                    return Poll::Ready(());
                }
                acq.step = AcquireStep::Loop(waiter);
                return Poll::Pending;
            }
            AcquireStep::Loop(waiter) => {
                let mut old_waker = (*waiter).waker.load(Acquire);
                let new_waker = RawWaker::encode(cx.waker().clone());
                loop {
                    match old_waker {
                        Cancelled => unreachable!(),
                        Waking => {
                            mem::drop(RawWaker::decode(new_waker));
                            self.freelist.free(waiter);
                            acq.step = AcquireStep::Poison;
                            return Poll::Ready(());
                        }
                        Sleeping(raw_waker) => {
                            if (*waiter).waker.compare_update_weak(
                                &mut old_waker, WaiterState::Sleeping(new_waker),
                                AcqRel, Acquire) {
                                mem::drop(RawWaker::decode(raw_waker));
                                return Poll::Pending;
                            }
                        }
                    }
                }
            }
            AcquireStep::Poison => unreachable!()
        }
    }

    unsafe fn acquire_drop(&self, acq: Pin<&mut Self::Acq>) {
        match acq.step {
            AcquireStep::Loop(waiter) => {
                match (*waiter).waker.swap(WaiterState::Cancelled, AcqRel) {
                    WaiterState::Waking => {
                        self.release((*waiter).amount);
                        self.freelist.free(waiter);
                    }
                    WaiterState::Sleeping(waker) => {
                        mem::drop(RawWaker::decode(waker));
                        let mut old_state = self.state.load(Acquire);
                        loop {
                            let mut state = old_state;
                            match state.mode {
                                Available | Queued => {
                                    state.mode = Locked;
                                    if self.state.compare_update_weak(
                                        &mut old_state, state, AcqRel, Acquire) {
                                        self.unlock();
                                        break;
                                    }
                                }
                                Locked => {
                                    state.mode = LockedDirty;
                                    if self.state.compare_update_weak(
                                        &mut old_state, state, AcqRel, Acquire) {
                                        break;
                                    }
                                }
                                LockedDirty => {
                                    break;
                                }
                            }
                        }
                    }
                    WaiterState::Cancelled => unreachable!(),
                }
            }
            AcquireStep::Enter { .. } => {}
            AcquireStep::Poison => {}
        }
    }

    fn try_acquire(&self, amount: usize) -> Result<(), WouldBlock> {
        let mut old_state = self.state.load(Acquire);
        loop {
            let mut state = old_state;
            if amount > state.available || state.mode != Available {
                return Err(WouldBlock);
            }
            state.available -= amount;
            if self.state.compare_update_weak(
                &mut old_state, state, AcqRel, Acquire) {
                return Ok(());
            }
        }
    }
}

