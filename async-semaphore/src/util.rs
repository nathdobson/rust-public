use std::task::{Poll, Context, Waker};
use std::pin::Pin;
use std::future::Future;
use std::mem::size_of;
use std::mem;

pub unsafe fn force_transmute<T, U>(value: T) -> U {
    assert_eq!(size_of::<T>(), size_of::<U>());
    let result = mem::transmute_copy(&value);
    mem::forget(value);
    result
}
