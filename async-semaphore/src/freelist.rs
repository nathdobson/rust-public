use std::cell::UnsafeCell;
use crate::atomic::{Atomic};
use std::ptr::null;
use std::sync::atomic::Ordering::{Acquire, Relaxed, AcqRel, Release};
use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::cmp::Ordering;
use std::marker::PhantomData;

pub type Allocator<T> = FreeList<T>;

pub struct Malloc<T>(PhantomData<T>);

impl<T> Malloc<T> {
    pub fn new() -> Self {
        Malloc(PhantomData)
    }
    pub unsafe fn allocate(&self, value: T) -> *const T {
        Box::into_raw(Box::new(value))
    }
    pub unsafe fn free(&self, ptr: *const T) -> T {
        *Box::from_raw(ptr as *mut T)
    }
}

#[repr(align(64))]
struct Node<T> {
    next: Atomic<*const Node<T>>,
    value: UnsafeCell<MaybeUninit<T>>,
}

pub struct FreeList<T> {
    head: Atomic<(*const Node<T>, usize)>
}

impl<T> FreeList<T> {
    pub fn new() -> Self {
        FreeList {
            head: Atomic::new((null(), 0)),
        }
    }

    pub unsafe fn allocate(&self, value: T) -> *const T {
        let mut old_free = self.head.load(Acquire);
        let result: *const Node<T>;
        loop {
            let (free, free_ver) = old_free;
            if free == null() || true {
                result = Box::into_raw(Box::new(
                    Node { next: Atomic::new(null()), value: UnsafeCell::new(MaybeUninit::uninit()) }));
                break;
            } else {
                let next = (*free).next.load(Relaxed);
                if self.head.compare_update_weak(
                    &mut old_free, (next, free_ver + 1), AcqRel, Acquire) {
                    result = free;
                    break;
                }
            }
        }
        (*result).next.store(!0 as *const Node<T>, Relaxed);
        (*(*result).value.get()).as_mut_ptr().write(value);
        let result = (*(*result).value.get()).as_ptr();
        //println!("Allocating {:?}", result);
        result
    }

    pub unsafe fn free(&self, ptr: *const T) -> T {
        let dummy: Node<T> = Node {
            next: Atomic::new(null()),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        };
        let offset = ((&dummy.value) as *const _ as *const u8).offset_from((&dummy) as *const _ as *const u8);
        let node = (ptr as *const u8).offset(-offset) as *const Node<T>;
        assert_eq!((*node).next.load(Relaxed), !0 as *mut Node<T>);
        let value = (*(*node).value.get()).as_mut_ptr().read();
        let mut old_head = self.head.load(Relaxed);
        loop {
            let (head, head_ver) = old_head;
            (*node).next.store(head, Relaxed);
            if self.head.compare_update_weak(
                &mut old_head, (node, head_ver + 1), Release, Relaxed) {
                break;
            }
        }
        value
    }
}

impl<T> Drop for FreeList<T> {
    fn drop(&mut self) {
        unsafe {
            let (mut ptr, _) = self.head.load(Relaxed);
            while ptr != null() {
                let next = (*ptr).next.load(Relaxed);
                Box::from_raw(ptr as *mut Node<T>);
                ptr = next;
            }
        }
    }
}

#[test]
fn test_freelist() {
    unsafe {
        let freelist = FreeList::new();
        println!("d");
        let arc = Arc::new(1);
        println!("c");
        let ptr = freelist.allocate(arc.clone());
        println!("b");
        let arc2 = freelist.free(ptr);
        println!("a");
        assert_eq!(arc, arc2);
        assert!(Arc::strong_count(&arc) == 2);
    }
}