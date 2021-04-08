use async_backtrace::{run_debug_server};
use async_backtrace::spawn;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    run_debug_server();
    loop {
        spawn(async move {
            sleep(Duration::from_millis(1000)).await;
        });
        sleep(Duration::from_millis(100)).await;
    }
}