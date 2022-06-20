use std::cell::{Cell, UnsafeCell};
use std::iter::Once;
use std::mem::{size_of, ManuallyDrop, MaybeUninit};
use std::ops::Deref;

use crate::cov_cell::CovCell;
use crate::{SafeLazy, SafeOnceCell};

pub struct CovLazy<T, F = fn() -> T> {
    cell: SafeOnceCell,
    init: CovCell<Option<F>>,
    value: CovCell<Option<T>>,
}

impl<T, F> CovLazy<T, F> {
    pub const fn new(f: F) -> Self {
        CovLazy {
            cell: SafeOnceCell::new(),
            init: CovCell::new(Some(f)),
            value: CovCell::new(None),
        }
    }
    pub fn into_inner(self) -> T
    where
        F: FnOnce() -> T,
    {
        if self.cell.into_inner().is_none() {
            (self.init.into_inner().unwrap())()
        } else {
            self.value.into_inner().unwrap()
        }
    }
}

impl<T: Default> CovLazy<T> {
    pub const fn const_default() -> Self { CovLazy::new(|| T::default()) }
}

impl<T, F: FnOnce() -> T> Deref for CovLazy<T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe {
            self.cell.get_or_init(|| {
                let value = (*self.init.as_inner().get()).take().unwrap()();
                *self.value.as_inner().get() = Some(value);
            });
            (*self.value.as_inner().get()).as_ref().unwrap()
        }
    }
}

unsafe impl<T: Send, F: Send> Send for CovLazy<T, F> {}

unsafe impl<T: Sync + Send, F: Send> Sync for CovLazy<T, F> {}

impl<T, F> From<T> for CovLazy<T, F> {
    fn from(x: T) -> Self {
        CovLazy {
            cell: SafeOnceCell::from(()),
            init: CovCell::new(None),
            value: CovCell::new(Some(x)),
        }
    }
}
