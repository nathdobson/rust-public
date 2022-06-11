use std::fmt::Debug;
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;

pub struct Client<I: Debug, O: Debug> {
    request: mpsc::Sender<I>,
    response: mpsc::Receiver<O>,
}

pub struct Server<I: Debug, O: Debug> {
    request: mpsc::Receiver<I>,
    response: mpsc::Sender<O>,
}

pub fn channel<I: Debug, O: Debug>() -> (Client<I, O>, Server<I, O>) {
    let (is, ir) = mpsc::channel();
    let (os, or) = mpsc::channel();
    (
        Client {
            request: is,
            response: or,
        },
        Server {
            request: ir,
            response: os,
        },
    )
}

impl<I: Debug, O: Debug> Client<I, O> {
    pub fn execute(&self, input: I) -> O {
        self.request.send(input).unwrap();
        self.response.recv_timeout(Duration::from_secs(1)).unwrap()
    }
}

impl<I: Debug, O: Debug> Server<I, O> {
    fn will(&self, callback: impl FnOnce(I) -> O) {
        self.response
            .send(callback(
                self.request.recv_timeout(Duration::from_secs(1)).unwrap(),
            ))
            .unwrap()
    }
    pub fn expect_timeout(&self) {
        match self.request.recv_timeout(Duration::from_millis(100)) {
            Err(RecvTimeoutError::Timeout) => return,
            Err(RecvTimeoutError::Disconnected) => panic!("Expected timeout, found disconnected"),
            Ok(x) => panic!("Expected timeout, found {:?}", x),
        }
    }
}

impl<I: Debug + PartialEq> Server<I, ()> {
    pub fn expect(&self, expected: I) {
        self.will(|input| {
            assert_eq!(input, expected);
        })
    }
    pub fn expect_and(&self, expected: I, callback: impl FnOnce()) {
        self.will(|input| {
            assert_eq!(input, expected);
            callback();
        })
    }
}

impl<I: Debug, O: Debug> Drop for Server<I, O> {
    fn drop(&mut self) {
        if !thread::panicking() {
            assert_eq!(
                self.request
                    .recv_timeout(Duration::from_secs(1))
                    .unwrap_err(),
                mpsc::RecvTimeoutError::Disconnected
            );
        }
    }
}
