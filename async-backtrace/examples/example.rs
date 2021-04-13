use async_backtrace::{traced_main, spawn};
use std::time::Duration;
use tokio::time::sleep;
use tokio::join;
use tokio::task::spawn_blocking;
use std::thread;
use std::fmt::Debug;

async fn sleepy() {
    sleep(Duration::from_secs(1000)).await;
}

async fn foo() {
    sleepy().await
}

async fn bar2() {
    join!(sleepy(),sleepy());
}

async fn bar1() {
    join!(bar2(), bar2());
}

fn baz() {
    thread::sleep(Duration::from_secs(1000));
}

#[inline(never)]
fn for_generic<T: Debug>(x: T) {
    thread::sleep(Duration::from_secs(1000));
    println!("{:?}", x);
}

fn main() {
    traced_main("127.0.0.1:9999".to_string(), async move {
        spawn(foo());
        spawn(bar1());
        spawn_blocking(baz);
        spawn_blocking(|| for_generic([10u8; 10]));
        spawn_blocking(|| for_generic({
            fn identity(x: usize) -> usize { x }
            identity as fn(usize) -> usize
        }));
        let () = sleepy().await;
    })
}