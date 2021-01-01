pub struct Heap {}

pub struct HeapGuard<'a> {
    heap: &'a Heap
}

impl Heap {
    pub fn new() -> Self {
        Heap {}
    }
    pub fn activate(&mut self) -> HeapGuard {
        unimplemented!()
    }
}

// struct SwappableAllocator;
//
// unsafe impl GlobalAlloc for Allocator {
//     unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
//         System.alloc(layout)
//     }
//
//     unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
//         System.dealloc(ptr, layout)
//     }
//
//     unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
//         System.alloc_zeroed(layout)
//     }
//
//     unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
//         System.realloc(ptr, layout, new_size)
//     }
// }