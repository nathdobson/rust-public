use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Mutex;
use std::thread;
use std::thread::ThreadId;

struct Atomic<T: Copy + Eq> {
    inner: Mutex<AtomicInner<T>>,
}

struct Store<T> {
    thread: ThreadId,
    time: usize,
    value: T,
}

struct AtomicInner<T> {
    stores: Vec<Store<T>>,
    last_release: HashMap<ThreadId, usize>,
}

struct ThreadHistory {
    now: usize,
    last_sync: HashMap<ThreadId, usize>,
}

thread_local! {
    static THREAD: RefCell<ThreadHistory> =
     RefCell::new(ThreadHistory{
        barriers : vec![SeqCst],
     });
}

impl<T: Copy + Eq> AtomicInner<T> {
    fn acquire(&mut self) {
        for (releaser, release) in inner.last_release {
            let sync = thread.last_sync.entry(releaser).or_insert(0);
            *sync = (*sync).max(release);
        }
    }
}

impl<T: Copy + Eq> Atomic<T> {
    fn compare_exchange(
        &self,
        current: T,
        new: T,
        success: Ordering,
        failure: Ordering,
    ) -> Result<T, T> {
        THREAD.with(|thread| {
            let thread_id = thread::current().id();
            let mut thread = thread.borrow_mut();
            let mut inner = self.inner.lock().unwrap();
            thread.now += 1;
            let old = inner.stores.last().unwrap().value;
            if old == current {
                inner.stores.push(Store {
                    thread: thread_id,
                    time: thread.now,
                    value: new,
                });
                if success == Ordering::Release || success == Ordering::AcqRel {
                    inner.last_release.insert(thread_id, thread.now);
                }
                if success == Ordering::Acquire || success == Ordering::AcqRel {
                    inner.acquire();
                }
                Ok(current)
            } else {
                Err(current)
            }
        })
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
