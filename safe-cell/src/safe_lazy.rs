use std::cell::Cell;
use std::ops::Deref;
use crate::SafeOnceCell;

pub struct SafeLazy<T, F = fn() -> T> {
    cell: SafeOnceCell<T>,
    init: Cell<Option<F>>,
}

impl<T, F> SafeLazy<T, F> {
    pub const fn new(f: F) -> Self {
        SafeLazy {
            cell: SafeOnceCell::new(),
            init: Cell::new(Some(f)),
        }
    }
}

impl<T: Default> SafeLazy<T> {
    pub const fn const_default() -> Self { SafeLazy::new(|| T::default()) }
}

impl<T, F: FnOnce() -> T> Deref for SafeLazy<T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target { self.cell.get_or_init(|| self.init.take().unwrap()()) }
}

unsafe impl<T: Send, F: Send> Send for SafeLazy<T, F> {}

unsafe impl<T: Sync + Send, F: Send> Sync for SafeLazy<T, F> {}
