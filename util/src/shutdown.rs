use std::process::{abort, exit};
use std::sync::Mutex;
use std::time::Duration;

use ctrlc;

use crate::cancel;
use crate::cancel::{JoinError, RecvError};

pub fn join_to_main(
    canceller: cancel::Canceller,
    receiver: cancel::Receiver,
    timeout: Duration,
) -> ! {
    let counter = Mutex::new(0);
    ctrlc::set_handler(move || {
        let mut lock = counter.lock().unwrap();
        match *lock {
            0 => canceller.cancel(),
            1..=3 => println!("Really skip cancellation?"),
            4 => exit(3),
            5..=8 => println!("Burninate?"),
            _ => abort(),
        }
        *lock += 1;
    })
    .unwrap();
    match receiver.recv() {
        Ok(()) => exit(0),
        Err(e) => match e {
            RecvError::Cancelling(joiner) => match joiner.join_timeout(timeout) {
                Ok(()) => exit(0),
                Err(e) => match e {
                    JoinError::Panic => exit(1),
                    JoinError::Empty(_) => exit(2),
                },
            },
            _ => exit(4),
        },
    }
}
