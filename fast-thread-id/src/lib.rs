#![feature(thread_local)]
#![feature(test)]
#![feature(bench_black_box)]
#![allow(unused_imports)]

use cfg_if::cfg_if;

#[thread_local]
static THREADID: u8 = 0;

cfg_if! {
    if #[cfg(all(target_arch = "wasm32"))]{
        #[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Copy, Clone)]
        pub struct FastThreadId();
    }else{
        #[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Copy, Clone)]
        pub struct FastThreadId(usize);
    }
}

impl FastThreadId {
    #[inline]
    pub fn get() -> FastThreadId {
        cfg_if! {
            if #[cfg(target_arch = "aarch64")]{
                // use std::arch::asm;
                // let x: usize;
                // unsafe { asm!("mrs {}, tpidr_el0", out(reg) x); }
                // FastThreadId(x)
                FastThreadId(&THREADID as *const u8 as usize)
            }else if #[cfg(all(target_arch = "x86_64"))]  {
                use std::arch::asm;
                let x: usize;
                unsafe { asm!("mov {}, gs", out(reg) x); }
                FastThreadId(x)
            }else if #[cfg(all(target_arch = "wasm32"))]{
                FastThreadId()
            }else{
                compile_error!("platform not supported", std::stringify())
            }
        }
    }
    #[inline]
    pub fn into_usize(self) -> usize {
        cfg_if! {
            if #[cfg(all(target_arch = "wasm32"))]{
                0
            }else{
                self.0
            }
        }
    }
}


#[cfg(test)]
mod test {
    extern crate test;

    use std::collections::HashSet;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use test::Bencher;
    use crate::FastThreadId;

    //
    // #[bench]
    // fn bench_slow_thread_id(b: &mut Bencher) {
    //     println!("{:?}", slow_thread_id as usize);
    //     b.iter(|| {
    //         for _ in 0..BENCH_COUNT {
    //             black_box(slow_thread_id());
    //         }
    //     })
    // }


    // #[test]
    // fn test_slow_thread_id() {
    //     test_thread_id(slow_thread_id)
    // }

    #[test]
    fn test_fast_thread_id() {
        test_thread_id()
    }

    fn test_thread_id() {
        #[thread_local]
        static THREAD_ID: usize = 0;
        fn thread_id() -> usize {
            &THREAD_ID as *const usize as usize
        }
        let count = 10;
        let barrier = Arc::new(Barrier::new(count));
        let diff = (0..count).map(|_| thread::spawn({
            let barrier = barrier.clone();
            move || {
                barrier.wait();
                let id = thread_id();
                let id2 = thread_id();
                assert_eq!(id, id2);
                barrier.wait();
                id
            }
        })).collect::<Vec<_>>().into_iter().map(|x| x.join().unwrap()).collect::<HashSet<_>>().len();
        assert_eq!(diff, count);
    }
}