use std::cell::UnsafeCell;
use std::mem;
use std::mem::{size_of, ManuallyDrop, MaybeUninit};
use std::ptr::drop_in_place;

pub union CovCell<T> {
    _value: ManuallyDrop<UnsafeCell<MaybeUninit<()>>>,
    repr: ManuallyDrop<T>,
}

impl<T> CovCell<T> {
    pub const fn new(x: T) -> Self {
        CovCell {
            repr: ManuallyDrop::new(x),
        }
    }
    pub fn as_inner(&self) -> &UnsafeCell<T> { unsafe { mem::transmute(self) } }
    pub fn as_inner_mut(&mut self) -> &mut UnsafeCell<T> { unsafe { mem::transmute(self) } }
    pub fn into_inner(self) -> T {
        unsafe {
            let result = mem::transmute_copy(&self);
            mem::forget(self);
            result
        }
    }
}

impl<T> Drop for CovCell<T> {
    fn drop(&mut self) { unsafe { drop_in_place(mem::transmute::<&mut Self, &mut T>(self)) } }
}

fn foo<'a: 'b, 'b>(x: CovCell<&'a u8>) -> CovCell<&'b u8> { x }
