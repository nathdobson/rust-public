use std::ops::{Bound, RangeBounds, Deref, Index};
use std::collections::VecDeque;
use std::io::{IoSlice, IoSliceMut};

pub struct SlicePair<T>(pub T, pub T);

impl<T, E> SlicePair<T> where T: Deref<Target=[E]> {
    pub fn indices<R: RangeBounds<usize>>(&self, r: R) -> (usize, usize, usize, usize) {
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
        if end <= self.0.len() {
            (start, end, 0, 0)
        } else if start >= self.0.len() {
            (self.0.len(), self.0.len(), start - self.0.len(), end - self.0.len())
        } else {
            (start, self.0.len(), 0, end - self.0.len())
        }
    }
    pub fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }
}

impl<T, E> Index<usize> for SlicePair<T> where T: Deref<Target=[E]> {
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

    pub fn index<R: RangeBounds<usize>>(&self, r: R) -> Self {
        let (i1, i2, i3, i4) = self.indices(r);
        SlicePair(&self.0[i1..i2], &self.1[i3..i4])
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
    pub fn index_mut<R: RangeBounds<usize>>(&mut self, r: R) -> SlicePair<&mut [T]> {
        let (i1, i2, i3, i4) = self.indices(r);
        SlicePair(&mut self.0[i1..i2], &mut self.1[i3..i4])
    }

    pub fn copy_from_slice(&mut self, other: &[T]) where T: Copy {
        self.0.copy_from_slice(&other[..self.0.len()]);
        self.1.copy_from_slice(&other[self.0.len()..]);
    }

    pub fn copy_from(&mut self, other: SlicePair<&[T]>) where T: Copy {
        self.index_mut(..other.0.len()).copy_from_slice(other.0);
        self.index_mut(other.0.len()..).copy_from_slice(other.1);
    }
}

impl<'a> SlicePair<&'a [u8]> {
    pub fn as_io(&self) -> [IoSlice; 2] {
        [IoSlice::new(self.0), IoSlice::new(self.1)]
    }
}

impl<'a> SlicePair<&'a mut [u8]> {
    pub fn as_io_mut(&mut self) -> [IoSliceMut; 2] {
        [IoSliceMut::new(&mut *self.0), IoSliceMut::new(&mut *self.1)]
    }
}
