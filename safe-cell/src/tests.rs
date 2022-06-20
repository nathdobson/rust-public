use std::panic::resume_unwind;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use crate::cov_lazy::CovLazy;
use crate::{SafeOnceCell, SafeOnceCellMap};

#[test]
fn test_simple() {
    let foo = SafeOnceCell::new();
    assert_eq!(&42u8, foo.get_or_init(|| 42u8));
}

#[test]
fn test_simple_cov() {
    let foo = CovLazy::new(|| 42u8);
    assert_eq!(42u8, *foo);
}

#[test]
fn test_simple_cov_cov() {
    struct MyFn3<'a> {
        value: &'a u8,
    }
    impl<'a> FnOnce<()> for MyFn3<'a> {
        type Output = &'a u8;
        extern "rust-call" fn call_once(self, args: ()) -> Self::Output { self.value }
    }
    type MyLazy3<'a> = CovLazy<&'a u8, MyFn3<'a>>;
    fn lazily3<'a>(x: &'a u8) -> MyLazy3<'a> { CovLazy::new(MyFn3 { value: x }) }
    fn is_cov<'a: 'b, 'b>(x: MyFn3<'a>) -> MyFn3<'b> { x }
}

fn parallel(threads: usize, f: impl 'static + Send + Sync + Fn()) {
    let barrier = Arc::new(Barrier::new(threads));
    let f = Arc::new(f);
    (0..threads)
        .map(|_| {
            thread::spawn({
                let barrier = barrier.clone();
                let f = f.clone();
                move || {
                    barrier.wait();
                    f();
                }
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .for_each(|x| {
            x.join().unwrap_or_else(|x| resume_unwind(x));
        });
}

#[test]
fn test_racy() {
    let cell = Arc::new(SafeOnceCell::new());
    parallel(1000, {
        let cell = cell.clone();
        move || {
            assert_eq!(cell.get_or_init(|| 42u8), &42u8);
        }
    });
    assert_eq!(&42, cell.get_or_init(|| panic!()));
}

#[test]
#[should_panic(expected = "Deadlock")]
fn test_reentrant() {
    let cell = SafeOnceCell::new();
    cell.get_or_init(|| *cell.get_or_init(|| 42));
}

#[test]
#[should_panic(expected = "Deadlock")]
fn test_reentrant_lazy() {
    static LAZY: CovLazy<!> = CovLazy::new(|| *LAZY);
    *LAZY;
}

#[test]
#[should_panic(expected = "Deadlock in initialization")]
fn test_racy_reentrant() {
    use rand::{thread_rng, Rng};

    let cell = Arc::new(SafeOnceCell::new());
    parallel(100, {
        let cell = cell.clone();
        move || {
            cell.get_or_init(|| {
                thread::sleep(Duration::from_millis(thread_rng().gen_range(0..100)));
                *cell.get_or_init(|| 42)
            });
        }
    });
}

#[test]
fn test_map() {
    let map = SafeOnceCellMap::<String, String>::new();
    assert_eq!(map.get_or_init("a", || "b".to_string()), "b");
    assert_eq!(map.get_or_init("a", || "c".to_string()), "b");
    assert_eq!(
        map.get_or_init("x", || {
            assert_eq!(map.get_or_init("y", || { "y".to_string() }), "y");
            "x".to_string()
        }),
        "x"
    );
}

#[test]
#[should_panic(expected = "Deadlock")]
fn test_map_reentrant() {
    let map = SafeOnceCellMap::<String, String>::new();
    map.get_or_init("x", || {
        map.get_or_init("x", || "x".to_string());
        "x".to_string()
    });
}
