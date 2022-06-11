use std::borrow::Borrow;
use std::mem;

use crate::Semaphore;

pub type SemaphoreGuard<'a> = SemaphoreGuardWith<&'a Semaphore>;

pub struct SemaphoreGuardWith<T: Borrow<Semaphore>> {
    semaphore: Option<T>,
    amount: usize,
}

impl<T: Borrow<Semaphore>> SemaphoreGuardWith<T> {
    pub fn new(semaphore: T, amount: usize) -> Self {
        SemaphoreGuardWith {
            semaphore: Some(semaphore),
            amount,
        }
    }
    pub fn forget(mut self) -> usize {
        self.semaphore = None;
        let amount = self.amount;
        mem::forget(self);
        amount
    }
}

impl<T: Borrow<Semaphore>> Drop for SemaphoreGuardWith<T> {
    fn drop(&mut self) {
        if let Some(semaphore) = self.semaphore.take() {
            semaphore.borrow().release(self.amount);
        }
    }
}
