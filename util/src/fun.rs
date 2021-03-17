use std::alloc::{Allocator, Layout, AllocError};
use std::ptr::NonNull;

struct NoopAllocator;

unsafe impl Allocator for NoopAllocator {
    fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, AllocError> { Err(AllocError) }
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {}
}

pub unsafe fn call_once_raw<T: FnOnce() + ?Sized>(ptr: *mut T) {
    (Box::from_raw_in(ptr, NoopAllocator))()
}

