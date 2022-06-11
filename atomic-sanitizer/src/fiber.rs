use std::ffi::c_void;
use std::mem;
use std::raw::TraitObject;
use std::sync::Arc;

use libc::{getcontext, makecontext, setcontext, swapcontext, ucontext_t};

use crate::heap::Heap;

struct Fiber {
    context: ucontext_t,
    stack: Vec<u8>,
    heap: Arc<Heap>,
}

// extern "C" fn call_box(f: Box<FnOnce()>) {
//     f()
// }

extern "C" fn foo(data: *mut (), vtable: *mut ()) {
    unsafe {
        (mem::transmute::<_, Box<dyn FnOnce()>>(TraitObject { data, vtable }))();
    }
}

impl Fiber {
    fn new(heap: Arc<Heap>, f: Box<dyn FnOnce()>) -> Self {
        unsafe {
            let mut context: ucontext_t = mem::zeroed();
            let mut stack = vec![0u8; 8192];
            getcontext(&mut context);
            context.uc_stack.ss_sp = stack.as_mut_ptr() as *mut c_void;
            context.uc_stack.ss_size = stack.len();
            let f = mem::transmute::<_, TraitObject>(f);
            makecontext(
                &mut context,
                mem::transmute(foo as extern "C" fn(*mut (), *mut ())),
                2,
                f.data,
                f.vtable,
            );
            Fiber {
                context,
                stack,
                heap,
            }
        }
    }
    fn poll(&mut self) {
        unsafe {
            let mut poll_ctx: ucontext_t = mem::zeroed();
            swapcontext(&mut poll_ctx, &mut self.context);
        }
    }
}

#[test]
fn test() {
    let foo = String::new();
    let mut fiber = Fiber::new(
        Arc::new(Heap::new()),
        Box::new(move || {
            println!("Hello! {:?}", foo);
        }),
    );
    fiber.poll();
    // unsafe {
    //     let mut context = std::mem::zeroed();
    //     let mut done = false;
    //     getcontext(&mut context as *mut ucontext_t);
    //     println!("Running");
    //     if !done {
    //         println!("First");
    //         func(&mut done, &context as *const ucontext_t);
    //         println!("Never");
    //     } else {
    //         println!("Second");
    //     }
    // }
}
//
// unsafe fn func(done: &mut bool, context: *const ucontext_t) {
//     *done = true;
//     setcontext(context);
// }
