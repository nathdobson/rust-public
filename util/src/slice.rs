use std::ops::{Bound, RangeBounds, Deref, Index};
use std::collections::VecDeque;
use std::io::{IoSlice, IoSliceMut};
use std::ptr;
use std::slice::SliceIndex;

#[derive(Debug)]
pub struct SlicePair<T>(pub T, pub T);

pub trait Slice {
    type Item;
    fn len(&self) -> usize;
    unsafe fn range<R: SliceIndex<[Self::Item], Output=[Self::Item]>>(self, r: R) -> Self;
}

pub unsafe trait SafeSlice: Slice {}

impl<T> Slice for *const [T] {
    type Item = T;

    fn len(&self) -> usize { <*const [T]>::len(*self) }

    unsafe fn range<R: SliceIndex<[Self::Item], Output=[Self::Item]>>(self, r: R) -> Self {
        r.get_unchecked(self)
    }
}

impl<T> Slice for *mut [T] {
    type Item = T;

    fn len(&self) -> usize { <*mut [T]>::len(*self) }

    unsafe fn range<R: SliceIndex<[Self::Item], Output=[Self::Item]>>(self, r: R) -> Self {
        r.get_unchecked_mut(self)
    }
}

impl<'a, T> Slice for &'a [T] {
    type Item = T;

    fn len(&self) -> usize { <[T]>::len(*self) }

    unsafe fn range<R: SliceIndex<[Self::Item], Output=[Self::Item]>>(self, r: R) -> Self {
        &self[r]
    }
}

unsafe impl<'a, T> SafeSlice for &'a [T] {}

impl<'a, T> Slice for &'a mut [T] {
    type Item = T;

    fn len(&self) -> usize { <[T]>::len(*self) }

    unsafe fn range<R: SliceIndex<[Self::Item], Output=[Self::Item]>>(self, r: R) -> Self {
        &mut self[r]
    }
}

unsafe impl<'a, T> SafeSlice for &'a mut [T] {}


impl<T: Slice> SlicePair<T> {
    pub fn range<R: RangeBounds<usize>>(self, r: R) -> Self where T: SafeSlice {
        unsafe { self.range_unsafe(r) }
    }
    pub unsafe fn range_unsafe<R: RangeBounds<usize>>(self, r: R) -> Self {
        let start = match r.start_bound() {
            Bound::Included(&x) => x,
            Bound::Excluded(&x) => x + 1,
            Bound::Unbounded => 0,
        };
        let end = match r.end_bound() {
            Bound::Included(&x) => x + 1,
            Bound::Excluded(&x) => x,
            Bound::Unbounded => self.0.len() + self.1.len(),
        };
        let (i1, i2, i3, i4) = if end <= self.0.len() {
            (start, end, 0, 0)
        } else if start >= self.0.len() {
            (self.0.len(), self.0.len(), start - self.0.len(), end - self.0.len())
        } else {
            (start, self.0.len(), 0, end - self.0.len())
        };
        SlicePair(self.0.range(i1..i2), self.1.range(i3..i4))
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<S, E> Index<usize> for SlicePair<S> where S: Deref<Target=[E]> {
    type Output = E;

    fn index(&self, index: usize) -> &Self::Output {
        if index < self.0.len() {
            &self.0[index]
        } else {
            &self.1[index - self.0.len()]
        }
    }
}

impl<'a, T> SlicePair<&'a [T]> {
    pub fn from_deque(x: &'a VecDeque<T>) -> Self {
        let (x1, x2) = x.as_slices();
        SlicePair(x1, x2)
    }
    pub fn copy_to(&self, to: &mut [T]) where T: Copy {
        to[..self.0.len()].copy_from_slice(self.0);
        to[self.0.len()..].copy_from_slice(self.1);
    }
}

impl<'a, T> SlicePair<&'a mut [T]> {
    pub fn from_deque_mut(x: &'a mut VecDeque<T>) -> Self {
        let (x1, x2) = x.as_mut_slices();
        SlicePair(x1, x2)
    }
    pub fn copy_from_slice(&mut self, other: &[T]) where T: Copy {
        self.0.copy_from_slice(&other[..self.0.len()]);
        self.1.copy_from_slice(&other[self.0.len()..]);
    }
    pub fn reborrow(&mut self) -> SlicePair<&mut [T]> {
        SlicePair(self.0, self.1)
    }
    pub fn copy_from(&mut self, other: SlicePair<&[T]>) where T: Copy {
        self.reborrow().range(..other.0.len()).copy_from_slice(other.0);
        self.reborrow().range(other.0.len()..).copy_from_slice(other.1);
    }
}

impl<T> SlicePair<*mut [T]> {
    pub unsafe fn as_mut<'a>(self) -> SlicePair<&'a mut [T]> {
        SlicePair(&mut *self.0, &mut *self.1)
    }
}

impl<T> SlicePair<*const [T]> {
    pub unsafe fn as_ref<'a>(self) -> SlicePair<&'a [T]> {
        SlicePair(&*self.0, &*self.1)
    }
}

pub unsafe fn vec_as_slice_raw<T>(vec: &Vec<T>) -> *const [T] {
    ptr::slice_from_raw_parts(vec.as_ptr(), vec.len())
}

pub unsafe fn raw_split_at_mut<T>(slice: *mut [T], len: usize) -> SlicePair<*mut [T]> {
    SlicePair((..len).get_unchecked_mut(slice), (len..).get_unchecked_mut(slice))
}

pub unsafe fn raw_split_at<T>(slice: *const [T], len: usize) -> SlicePair<*const [T]> {
    SlicePair((..len).get_unchecked(slice), (len..).get_unchecked(slice))
}

impl<'a> SlicePair<&'a [u8]> {
    pub fn as_io(self) -> [IoSlice<'a>; 2] {
        [IoSlice::new(self.0), IoSlice::new(self.1)]
    }
}

impl<'a> SlicePair<&'a mut [u8]> {
    pub fn as_io_mut(&mut self) -> [IoSliceMut; 2] {
        [IoSliceMut::new(&mut *self.0), IoSliceMut::new(&mut *self.1)]
    }
}
