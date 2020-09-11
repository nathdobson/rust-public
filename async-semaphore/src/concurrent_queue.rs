use std::fmt::Debug;
use std::fmt::Write;
use std::ops::Deref;
use std::sync::atomic::{AtomicU128, AtomicU64, Ordering, AtomicUsize, AtomicU32};
use std::sync::atomic::AtomicU8;

use crate::atomic::Atomic;
use std::cell::UnsafeCell;
use std::mem::{MaybeUninit, align_of};
use std::{thread, mem};
use std::time::Duration;
use std::ptr::null;
use std::marker::PhantomData;

pub struct ConcurrentQueue<T: Default> {
    front: Atomic<(*const Node<T>, usize), AtomicU128>,
    back: Atomic<(*const Node<T>, usize), AtomicU128>,
    free: Atomic<(PtrSeq<Node<T>>, usize), AtomicU128>,
}

struct Node<T> {
    refcount: AtomicUsize,
    next: Atomic<PtrSeq<Node<T>>, AtomicU64>,
    value: UnsafeCell<MaybeUninit<T>>,
}

pub struct NodeRef<'a, T: Default> {
    list: &'a ConcurrentQueue<T>,
    link: *const Node<T>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
struct PtrSeq<T>(usize, PhantomData<T>);

impl<T> Copy for PtrSeq<T> {}

impl<T> Clone for PtrSeq<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PtrSeq<T> {
    fn as_ptr(&self) -> Option<*const T> {
        if self.0 & 1 == 1 {
            None
        } else {
            Some(self.0 as *const T)
        }
    }
    fn ptr(ptr: *const T) -> Self {
        PtrSeq(ptr as usize, PhantomData)
    }
    fn seq(seq: usize) -> Self {
        PtrSeq(seq * 2 + 1, PhantomData)
    }
}

impl<T: Default> ConcurrentQueue<T> {
    pub fn new() -> Self {
        let n1: *const Node<T> = Box::into_raw(Box::new(Node {
            refcount: AtomicUsize::new(2),
            value: UnsafeCell::new(MaybeUninit::new(T::default())),
            next: Atomic::new(PtrSeq::seq(1)),
        }));
        let n2: *const Node<T> = Box::into_raw(Box::new(Node {
            refcount: AtomicUsize::new(1),
            value: UnsafeCell::new(MaybeUninit::new(T::default())),
            next: Atomic::new(PtrSeq::ptr(n1)),
        }));
        let front = Atomic::new((n2, 0));
        let back = Atomic::new((n1, 1));
        let free = Atomic::new((PtrSeq::seq(0), 0));
        ConcurrentQueue { front, back, free }
    }
    pub fn push_back(&self, value: T) -> (NodeRef<'_, T>, NodeRef<'_, T>) {
        unsafe {
            let node: *const Node<T>;
            loop {
                let (free, free_ver)
                    = self.free.load(Ordering::Acquire);
                if let Some(free) = free.as_ptr() {
                    let next = (*free).next.load(Ordering::Acquire);
                    if self.free.compare_exchange_weak(
                        (PtrSeq::ptr(free), free_ver),
                        (next, free_ver + 1),
                        Ordering::AcqRel, Ordering::Acquire).is_ok() {
                        node = free;
                        break;
                    }
                } else {
                    node = Box::into_raw(Box::new(Node {
                        refcount: AtomicUsize::new(0),
                        value: UnsafeCell::new(MaybeUninit::uninit()),
                        next: Atomic::new(PtrSeq::seq(0)),
                    }));
                    break;
                }
            }
            (*(*node).value.get()).as_mut_ptr().write(value);
            (*node).refcount.store(4, Ordering::Relaxed);
            loop {
                let (back, seq) = self.back.load(Ordering::Acquire);
                (*node).next.store(PtrSeq::seq(seq + 1), Ordering::Release);
                match (*back).next.compare_exchange_weak(PtrSeq::seq(seq),
                                                         PtrSeq::ptr(node),
                                                         Ordering::AcqRel,
                                                         Ordering::Acquire) {
                    Ok(_) => return (NodeRef { list: self, link: back }, NodeRef { list: self, link: node }),
                    Err(next) => {
                        if let Some(next) = next.as_ptr() {
                            self.back.compare_exchange_weak(
                                (back, seq),
                                (next, seq + 1),
                                Ordering::AcqRel,
                                Ordering::Acquire).ok();
                        }
                    }
                }
            }
        }
    }
    pub fn pop_front(&self) -> Option<NodeRef<'_, T>> {
        unsafe {
            loop {
                let (first, first_ver) = self.front.load(Ordering::Acquire);
                let second = (*first).next.load(Ordering::Acquire);
                let second = if let Some(ptr) = second.as_ptr() { ptr } else { continue; };
                let third = (*second).next.load(Ordering::Acquire);
                if let Some(third) = third.as_ptr() {
                    if self.front.compare_exchange_weak(
                        (first, first_ver),
                        (second, first_ver + 1),
                        Ordering::AcqRel,
                        Ordering::Acquire).is_ok() {
                        self.downref(first);
                        return Some(NodeRef { list: self, link: third });
                    }
                } else {
                    if self.front.compare_exchange_weak(
                        (first, first_ver),
                        (first, first_ver + 1),
                        Ordering::AcqRel, Ordering::Acquire).is_ok() {
                        return None;
                    }
                }
            }
        }
    }
    unsafe fn downref(&self, link: *const Node<T>) {
        let old = (*link).refcount.fetch_sub(1, Ordering::AcqRel);
        if old == 1 {
            (*(*link).value.get()).as_mut_ptr().drop_in_place();
            loop {
                let (free, free_ver) =
                    self.free.load(Ordering::Acquire);
                (*link).next.store(free, Ordering::Relaxed);
                if self.free.compare_exchange_weak((free, free_ver),
                                                   (PtrSeq::ptr(link), free_ver + 1),
                                                   Ordering::AcqRel,
                                                   Ordering::Acquire).is_ok() {
                    break;
                }
            }
        }
    }
    pub unsafe fn debug_string(&self) -> String where T: Debug {
        let mut output = String::new();
        write!(&mut output, "[").unwrap();
        let (front, _) = self.front.load(Ordering::Relaxed);
        let mut link = front;
        loop {
            if link != front {
                write!(&mut output, "{:?} ;", (*link).value).unwrap();
            }
            if link == self.back.load(Ordering::Relaxed).0 {
                write!(&mut output, " [back]").unwrap();
            }
            if let Some(next) = (*link).next.load(Ordering::Relaxed).as_ptr() {
                link = next;
            } else {
                break;
            }
        }
        write!(&mut output, "]").unwrap();
        output
    }
}

impl<T: Default> Deref for NodeRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*(*(*self.link).value.get()).as_ptr()
        }
    }
}

impl<T: Default> Drop for NodeRef<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.list.downref(self.link);
        }
    }
}

unsafe impl<T: Send + Sync + Default> Send for ConcurrentQueue<T> {}

unsafe impl<T: Send + Sync + Default> Sync for ConcurrentQueue<T> {}

impl<T: Default> Drop for ConcurrentQueue<T> {
    fn drop(&mut self) {
        unsafe {
            let (mut link, _) = self.front.load(Ordering::Relaxed);
            loop {
                (*(*link).value.get()).as_mut_ptr().drop_in_place();
                let link2 = (*link).next.load(Ordering::Relaxed);
                Box::from_raw(link as *mut Node<T>);
                link = if let Some(ptr) = link2.as_ptr() { ptr } else { break; }
            }
            let (mut link, _) = self.free.load(Ordering::Relaxed);
            while let Some(ptr) = link.as_ptr() {
                let link2 = (*ptr).next.load(Ordering::Relaxed);
                Box::from_raw(ptr as *mut Node<T>);
                link = link2;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::{VecDeque, HashMap, HashSet};
    use std::cell::{Cell};
    use crate::concurrent_queue::ConcurrentQueue;
    use std::{mem, thread, fmt};
    use rand_xorshift::XorShiftRng;
    use rand::SeedableRng;
    use rand::Rng;
    use std::sync::{Arc, Mutex, Barrier, mpsc};
    use itertools::Itertools;
    use std::sync::atomic::AtomicUsize;
    use std::fmt::{Debug, Formatter};
    use lazy_static::lazy_static;
    use std::ops::Deref;
    use test::Bencher;
    use crossbeam::queue::SegQueue;
    use std::sync::mpsc::channel;

    #[test]
    fn test_simple() {
        unsafe {
            let list = ConcurrentQueue::<usize>::new();
            println!("{}", list.debug_string());
            list.push_back(1);
            println!("{}", list.debug_string());
            assert_eq!(*list.pop_front().unwrap(), 1);
            println!("{}", list.debug_string());
            list.push_back(2);
            println!("{}", list.debug_string());
            assert_eq!(*list.pop_front().unwrap(), 2);
            println!("{}", list.debug_string());
        }
    }

    #[test]
    fn test_none() {
        ConcurrentQueue::<usize>::new();
    }

    #[test]
    fn test_vs() {
        let mut deque = VecDeque::new();
        let list = ConcurrentQueue::<Box<usize>>::new();
        let mut refs = Vec::new();
        let mut rng = XorShiftRng::seed_from_u64(123321333);
        for push in 0..10000 {
            if rng.gen_bool(0.5) {
                if rng.gen_bool(0.5) {
                    println!("pushing {}", push);
                    deque.push_back(push);
                    refs.push(list.push_back(Box::new(push)));
                } else {
                    let expected_popped = deque.pop_front();
                    println!("popping {:?}", expected_popped);
                    let actual_popped = list.pop_front();
                    match expected_popped {
                        None => assert!(actual_popped.is_none()),
                        Some(expected_popped) => {
                            assert_eq!(expected_popped, **actual_popped.unwrap())
                        }
                    }
                }
            } else {
                println!("retaining");
                refs.retain(|_| { rng.gen_bool(0.5) });
            }
        }
        mem::drop(refs);
        mem::drop(list);
    }

    #[test]
    fn test_parallel_many() {
        for i in 0..10 {
            println!("{}", i);
            test_parallel_once(i);
        }
    }

    fn test_parallel_once(seed: u64) {
        #[derive(Copy, Clone, Default, Ord, PartialOrd, Eq, PartialEq, Hash)]
        struct Step {
            thread: usize,
            seq: usize,
        }
        lazy_static! {
            static ref GLOBAL_SEQ: Mutex<HashMap<Step,usize>> = Mutex::new(HashMap::new());
        }
        GLOBAL_SEQ.lock().unwrap().clear();

        impl Debug for Step {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                if let Some(global_seq) = GLOBAL_SEQ.lock().unwrap().get(&self) {
                    write!(f, "{}({}.{})", global_seq, self.thread, self.seq)
                } else {
                    write!(f, "({}.{})", self.thread, self.seq)
                }
            }
        }

        #[derive(Copy, Clone, Debug)]
        enum Event {
            Push(Option<Step>, Step),
            PopFail(Step),
            Pop(Step, Step),
        }
        let list = Arc::new(ConcurrentQueue::<Box<Step>>::new());
        let threads = 50;
        let barrier = Arc::new(Barrier::new(threads));
        let events: Vec<Vec<Event>> = (0..threads).map(|thread| {
            let list = list.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut refs = Vec::new();
                let mut events = vec![];
                let mut rng = XorShiftRng::seed_from_u64(seed);
                barrier.wait();
                for seq in 1..1000 {
                    if rng.gen_bool(1.0) {
                        let step = Step { thread, seq };
                        if rng.gen_bool(0.5) {
                            let (prev, pushed) = list.push_back(Box::new(step));
                            if **prev == Step::default() {
                                events.push(Event::Push(None, **pushed));
                            } else {
                                events.push(Event::Push(Some(**prev), **pushed));
                            }
                            refs.push(pushed);
                            refs.push(prev);
                        } else {
                            if let Some(popped) = list.pop_front() {
                                events.push(Event::Pop(**popped, step));
                                refs.push(popped);
                            } else {
                                events.push(Event::PopFail(step));
                            }
                        }
                    } else {
                        refs.retain(|_| { rng.gen_bool(0.5) });
                    }
                }
                events
            })
        }).collect::<Vec<_>>().into_iter().map(|x| x.join().unwrap()).collect();
        let mut push_next = HashMap::new();
        let mut first = None;
        for thread in events.iter() {
            for event in thread {
                match event {
                    Event::Push(Some(old), new) => {
                        push_next.insert(*old, *new);
                    }
                    Event::Push(None, new) => {
                        first = Some(*new);
                    }
                    _ => {}
                }
            }
        }
        if first.is_some() {
            let mut step = first;
            for i in 0..(push_next.len() + 1) {
                GLOBAL_SEQ.lock().unwrap().insert(step.unwrap(), i);
                step = push_next.get(&step.unwrap()).cloned();
            }
        }

        let mut sent = HashSet::new();
        let mut received = HashSet::new();
        for thread in events.iter() {
            for event in thread {
                match event {
                    Event::Push(_old, new) => { sent.insert(*new); }
                    Event::PopFail(_) => {}
                    Event::Pop(push, _) => { received.insert(*push); }
                }
            }
        }
        while let Some(remainder) = list.pop_front() {
            received.insert(**remainder);
        }
        assert_eq!(sent, received);

        let mut iters = events.iter().map(|x| x.iter().peekable()).collect::<Vec<_>>();
        let mut next_push = first;
        let mut next_pop = first;
        let mut size = 0;
        let mut order = vec![];
        loop {
            if size == 0 {
                if iters.iter_mut().any(|iter| {
                    if let Some(Event::PopFail(_)) = iter.peek() {
                        order.push(iter.next().unwrap());
                        true
                    } else {
                        false
                    }
                }) {
                    continue;
                }
            } else {
                if iters.iter_mut().any(|iter| {
                    if let Some(Event::Pop(pushed, _popped)) = iter.peek() {
                        if next_pop == Some(*pushed) {
                            next_pop = push_next.get(&next_pop.unwrap()).cloned();
                            order.push(iter.next().unwrap());
                            size -= 1;
                            return true;
                        }
                    }
                    false
                }) {
                    continue;
                }
            }
            if iters.iter_mut().any(|iter| {
                if let Some(Event::Push(_old, new)) = iter.peek() {
                    if next_push == Some(*new) {
                        next_push = push_next.get(&next_push.unwrap()).cloned();
                        order.push(iter.next().unwrap());
                        size += 1;
                        return true;
                    }
                }
                false
            }) {
                continue;
            }
            if next_push.is_none() && iters.iter_mut().all(|iter| iter.peek().is_none()) {
                break;
            }
            println!("next_push = {:?}", next_push);
            println!("next_pop = {:?}", next_pop);
            println!("size = {:?}", size);
            for x in iters.iter_mut() {
                println!("{:?}", x.peek());
            }
            for event in order.iter().enumerate() {
                println!("{:?}", event);
            }
            for (index, thread) in events.iter().enumerate() {
                println!("thread {}", index);
                for event in thread {
                    match event {
                        Event::Push(old, new) =>
                            println!("push {:?} {:?}", old, new),
                        Event::PopFail(s) =>
                            println!("pop {:?}", s),
                        Event::Pop(push, pop) =>
                            println!("pop {:?} {:?}", push, pop),
                    }
                }
            }

            panic!("Cannot proceed.");
        }
    }

    trait IsQueue: Send + Sync {
        fn new() -> Self;
        fn push_back(&self, value: usize);
        fn pop_front(&self) -> Option<usize>;
    }

    impl IsQueue for ConcurrentQueue<usize> {
        fn new() -> Self {
            ConcurrentQueue::new()
        }

        fn push_back(&self, value: usize) {
            self.push_back(value);
        }

        fn pop_front(&self) -> Option<usize> {
            match self.pop_front() {
                None => None,
                Some(x) => Some(*x),
            }
        }
    }

    impl IsQueue for Mutex<VecDeque<Arc<usize>>> {
        fn new() -> Self {
            Mutex::new(VecDeque::new())
        }

        fn push_back(&self, value: usize) {
            self.lock().unwrap().push_back(Arc::new(value));
        }

        fn pop_front(&self) -> Option<usize> {
            self.lock().unwrap().pop_front().map(|x| *x)
        }
    }

    impl IsQueue for SegQueue<Arc<usize>> {
        fn new() -> Self {
            SegQueue::new()
        }

        fn push_back(&self, value: usize) {
            self.push(Arc::new(value))
        }

        fn pop_front(&self) -> Option<usize> {
            self.pop().ok().map(|x| *x)
        }
    }

    impl IsQueue for ::concurrent_queue::ConcurrentQueue<Arc<usize>> {
        fn new() -> Self {
            ::concurrent_queue::ConcurrentQueue::unbounded()
        }

        fn push_back(&self, value: usize) {
            self.push(Arc::new(value)).unwrap();
        }

        fn pop_front(&self) -> Option<usize> {
            self.pop().ok().map(|x| *x)
        }
    }

    fn run_bench<Q: IsQueue + 'static>() -> usize {
        let queue = Arc::new(Q::new());
        let threads = 4;
        let max = 10000;
        let barrier = Arc::new(Barrier::new(threads));
        let mut out = (0..threads).map(|thread| {
            let queue = queue.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                let mut sum = 0;
                for seq in 1..max {
                    if thread % 2 == 0 {
                        queue.push_back(seq);
                        sum += seq;
                    } else {
                        if let Some(pop) = queue.pop_front() {
                            sum += pop;
                        }
                    }
                }
                sum
            })
        }).collect::<Vec<_>>().into_iter().map(|x| x.join().unwrap()).sum::<usize>();
        while let Some(x) = queue.pop_front() {
            out += x;
        }
        assert_eq!(out, threads * (0..max).sum::<usize>());
        out
    }

    #[bench]
    fn benchmark_self(b: &mut Bencher) {
        b.iter(|| run_bench::<ConcurrentQueue<usize>>());
    }

    #[bench]
    fn benchmark_mutex(b: &mut Bencher) {
        b.iter(|| run_bench::<Mutex<VecDeque<Arc<usize>>>>());
    }

    #[bench]
    fn benchmark_crossbeam(b: &mut Bencher) {
        b.iter(|| run_bench::<SegQueue<Arc<usize>>>());
    }

    #[bench]
    fn benchmark_concurrent(b: &mut Bencher) {
        b.iter(|| run_bench::<::concurrent_queue::ConcurrentQueue<Arc<usize>>>());
    }
}
