use async_backtrace::{run_debug_server};
use async_backtrace::spawn;
use std::time::Duration;
use tokio::time::sleep;
use tokio::join;
use tokio::task::spawn_blocking;
use std::thread;

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

fn baz(){
    thread::sleep(Duration::from_secs(1000));
}

#[tokio::main]
async fn main() {
    run_debug_server(9999);
    spawn(foo());
    spawn(bar1());
    spawn_blocking(baz);
    let () = sleepy().await;
}