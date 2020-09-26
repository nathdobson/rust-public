mod waker;

use std::sync::atomic::{Ordering, AtomicUsize};
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
use crate::atomic::{Atomic, usize2, AtomicPackable, usize1};
use std::sync::atomic::Ordering::{SeqCst, Relaxed, AcqRel, Acquire, Release};
use crate::shared_swcas::ReleaseMode::{Unlocked, Locked, LockedDirty};
use crate::freelist::Allocator;
use crate::{ReleaseGuard, TryAcquireError, AcquireRelease, Disconnected};
use crate::WouldBlock;
use crate::shared_swcas::waker::{AtomicWaker, FinishedError, CancelledError};
use crate::shared_swcas::waker::PendingError;
use crate::shared_swcas::AcquireState::{Available, Queued};

pub struct SemaphoreImpl {
    acquire: Atomic<AcquireState>,
    release: Atomic<ReleaseState>,
    front: UnsafeCell<*const Waiter>,
    freelist: Allocator<Waiter>,
}

#[repr(align(64))]
pub struct Waiter {
    waker: AtomicWaker,
    next: UnsafeCell<*const Waiter>,
    remaining: UnsafeCell<usize>,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
enum ReleaseMode {
    Unlocked,
    Locked,
    LockedDirty,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct ReleaseState {
    releasable: usize,
    mode: ReleaseMode,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
enum AcquireState {
    Queued(*const Waiter),
    Available(usize),
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

impl Unpin for AcquireImpl {}

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
                write!(f, " {:?} {:?}", (*self.0).remaining, DebugPtr(*(*self.0).next.get()))?;
            }
            Ok(())
        }
    }
}

impl AtomicPackable for ReleaseState {
    type Raw = usize1;
    unsafe fn encode(val: Self) -> Self::Raw {
        ((val.releasable << 2) | (match val.mode {
            Unlocked => 0,
            Locked => 1,
            LockedDirty => 2,
        })) as usize1
    }
    unsafe fn decode(val: Self::Raw) -> Self {
        ReleaseState {
            releasable: (val >> 2) as usize,
            mode: match val & 3 {
                0 => Unlocked,
                1 => Locked,
                2 => LockedDirty,
                _ => unreachable!()
            },
        }
    }
}

impl AtomicPackable for AcquireState {
    type Raw = usize1;
    unsafe fn encode(val: Self) -> Self::Raw {
        match val {
            Queued(back) => back as usize1,
            Available(available) => ((available << 1) | 1) as usize1,
        }
    }
    unsafe fn decode(val: Self::Raw) -> Self {
        if val & 1 == 1 {
            Available((val >> 1) as usize)
        } else {
            Queued(val as *const Waiter)
        }
    }
}

impl Drop for SemaphoreImpl {
    fn drop(&mut self) {
        unsafe {
            let back = match self.acquire.load(Relaxed) {
                Queued(back) => back,
                Available(_) => null(),
            };
            for &(mut ptr) in &[*self.front.get(), back] {
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
            w.finish()
        }
    }
}

impl SemaphoreImpl {
    unsafe fn clear_dirty(&self) {
        let mut old_release = self.release.load(Acquire);
        loop {
            let mut release = old_release;
            if release.mode == LockedDirty {
                release.mode = Locked;
                if self.release.compare_update_weak(
                    &mut old_release, release,
                    AcqRel, Acquire) {
                    break;
                }
            } else {
                break;
            }
        }
    }

    unsafe fn consume(&self, amount: usize) -> bool {
        let mut old_release = self.release.load(Acquire);
        loop {
            let mut release = old_release;
            release.releasable -= amount;
            if self.release.compare_update_weak(
                &mut old_release, release,
                AcqRel, Acquire) {
                return release.mode == Unlocked;
            }
        }
    }

    unsafe fn flip(&self) {
        let mut old_release = self.release.load(Acquire);
        let mut acquire = self.acquire.load(Acquire);
        loop {
            match acquire {
                Queued(back) if back == null() => {
                    if self.acquire.compare_update_weak(
                        &mut acquire, Available(old_release.releasable), AcqRel, Acquire) {
                        self.consume(old_release.releasable);
                        return;
                    }
                }
                Available(available) => {
                    if self.acquire.compare_update_weak(
                        &mut acquire, Available(available + old_release.releasable), AcqRel, Acquire) {
                        self.consume(old_release.releasable);
                        return;
                    }
                }
                Queued(mut back) => {
                    if self.acquire.compare_update_weak(
                        &mut acquire, Queued(null()), AcqRel, Acquire) {
                        while back != null() {
                            let next = *(*back).next.get();
                            *(*back).next.get() = *(*self).front.get();
                            *self.front.get() = back;
                            back = next;
                        }
                        return;
                    }
                }
            }
        }
    }

    unsafe fn try_pop(&self) -> bool {
        let mut old_release = self.release.load(Acquire);
        let front = *self.front.get();
        let remaining = *(*front).remaining.get();
        if remaining <= old_release.releasable {
            *self.front.get() = *(*front).next.get();
            if (*front).waker.finish().is_ok() {
                loop {
                    let mut release = old_release;
                    release.releasable -= remaining;
                    if self.release.compare_update_weak(
                        &mut old_release, release, AcqRel, Acquire) {
                        break;
                    }
                }
            } else {
                //println!("Completed error {:?} ", front);
                self.freelist.free(front);
            }
            return true;
        }
        if (*front).waker.is_cancelled() {
            *self.front.get() = *(*front).next.get();
            //println!("Cancelled {:?} ", front);
            self.freelist.free(front);
            return true;
        }
        false
    }

    unsafe fn try_unlock(&self) -> bool {
        let mut old_release = self.release.load(Acquire);
        loop {
            let mut release = old_release;
            if release.mode == Locked {
                release.mode = Unlocked;
            }
            if self.release.compare_update_weak(
                &mut old_release, release,
                AcqRel, Acquire) {
                return release.mode == Unlocked;
            }
        }
    }

    unsafe fn unlock(&self) {
        loop {
            self.clear_dirty();
            if *self.front.get() == null() {
                self.flip();
            }
            if *self.front.get() != null() {
                if self.try_pop() {
                    continue;
                }
            }
            if self.try_unlock() {
                return;
            }
        }
    }
}

impl AcquireRelease for SemaphoreImpl {
    type Acq = AcquireImpl;

    fn new(initial: usize) -> Self {
        SemaphoreImpl {
            acquire: Atomic::new(Available(initial)),
            release: Atomic::new(ReleaseState { releasable: 0, mode: ReleaseMode::Unlocked }),
            front: UnsafeCell::new(null()),
            freelist: Allocator::new(),
        }
    }

    fn release(&self, amount: usize) {
        unsafe {
            let mut old_state = self.release.load(Acquire);
            loop {
                let mut state = old_state;
                state.releasable += amount;
                match old_state.mode {
                    Unlocked => {
                        state.mode = Locked;
                        if self.release.compare_update_weak(
                            &mut old_state, state, AcqRel, Acquire) {
                            self.unlock();
                            return;
                        }
                    }
                    Locked | LockedDirty => {
                        state.mode = LockedDirty;
                        if self.release.compare_update_weak(
                            &mut old_state, state, AcqRel, Acquire) {
                            return;
                        }
                    }
                }
            }
        }
    }


    unsafe fn acquire_new(&self, amount: usize) -> Self::Acq {
        AcquireImpl { amount, step: AcquireStep::Enter }
    }

    unsafe fn acquire_poll(&self, mut acq: Pin<&mut AcquireImpl>, cx: &mut Context<'_>) -> Poll<()> {
        match acq.step {
            AcquireStep::Enter => {
                let mut old_state = self.acquire.load(Acquire);
                let mut waiter: *const Waiter = null();
                loop {
                    match old_state {
                        Queued(back) => {
                            if waiter == null() {
                                waiter = self.freelist.allocate(Waiter {
                                    waker: AtomicWaker::new(cx.waker().clone()),
                                    next: UnsafeCell::new(null()),
                                    remaining: UnsafeCell::new(0),
                                });
                            }
                            *(*waiter).next.get() = back;
                            *(*waiter).remaining.get() = acq.amount;
                            if self.acquire.compare_update_weak(
                                &mut old_state, Queued(waiter), AcqRel, Acquire) {
                                acq.step = AcquireStep::Loop(waiter);
                                return Poll::Pending;
                            }
                        }
                        Available(available) => {
                            if acq.amount <= available {
                                if self.acquire.compare_update_weak(
                                    &mut old_state, Available(available - acq.amount), AcqRel, Acquire) {
                                    if waiter != null() {
                                        //println!("Free misallocated {:?}", waiter);
                                        self.freelist.free(waiter);
                                        waiter = null();
                                    }
                                    acq.step = AcquireStep::Poison;
                                    return Poll::Ready(());
                                }
                            } else {
                                if waiter == null() {
                                    waiter = self.freelist.allocate(Waiter {
                                        waker: AtomicWaker::new(cx.waker().clone()),
                                        next: UnsafeCell::new(null()),
                                        remaining: UnsafeCell::new(0),
                                    });
                                }
                                *(*waiter).next.get() = null();
                                *(*waiter).remaining.get() = acq.amount - available;
                                if self.acquire.compare_update_weak(
                                    &mut old_state, Queued(waiter), AcqRel, Acquire) {
                                    acq.step = AcquireStep::Loop(waiter);
                                    return Poll::Pending;
                                }
                            }
                        }
                    }
                }
            }
            AcquireStep::Loop(waiter) => {
                match (*waiter).waker.poll(cx.waker()) {
                    Ok(()) => {
                        //println!("Freeing ready {:?}", waiter);
                        self.freelist.free(waiter);
                        acq.step = AcquireStep::Poison;
                        Poll::Ready(())
                    }
                    Err(PendingError) => Poll::Pending,
                }
            }
            AcquireStep::Poison => unreachable!()
        }
    }

    unsafe fn acquire_drop(&self, acq: Pin<&mut Self::Acq>) {
        match acq.step {
            AcquireStep::Loop(waiter) => {
                if (*waiter).waker.cancel().contains_err(&FinishedError) {
                    //println!("Freeing on cancel {:?}", waiter);
                    self.release(acq.amount);
                    self.freelist.free(waiter);
                }
            }
            AcquireStep::Enter { .. } => {}
            AcquireStep::Poison => {}
        }
    }

    fn try_acquire(&self, amount: usize) -> Result<(), WouldBlock> {
        let mut old_state = self.acquire.load(Acquire);
        loop {
            match old_state {
                Queued(_) => return Err(WouldBlock),
                Available(available) => {
                    if amount <= available {
                        if self.acquire.compare_update_weak(
                            &mut old_state, Available(available - amount), AcqRel, Acquire) {
                            return Ok(());
                        }
                    } else {
                        return Err(WouldBlock);
                    }
                }
            }
        }
    }
}

