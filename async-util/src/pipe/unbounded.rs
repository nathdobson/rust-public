use std::sync::atomic::AtomicPtr;
use crate::waker::AtomicWaker;
use async_std::channel::{Sender, Receiver, unbounded};
use crate::pipe::bounded;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, FutureExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::io;
use async_channel::RecvError;
use futures_util::task::noop_waker;
use futures::task::{noop_waker_ref, SpawnExt};
use std::io::{Error, Write};
use futures::Stream;
use futures::executor::LocalPool;
use futures::join;

pub struct PipeWrite {
    write: bounded::PipeWrite,
    sender: Sender<bounded::PipeRead>,
}

pub struct PipeRead {
    read: bounded::PipeRead,
    receiver: Receiver<bounded::PipeRead>,
}

pub fn pipe() -> (PipeWrite, PipeRead) {
    let (sender, receiver) = unbounded();
    let (write, read) = bounded::pipe(0);
    (PipeWrite { write, sender }, PipeRead { read, receiver })
}

impl Unpin for PipeWrite {}

impl Unpin for PipeRead {}

impl AsyncRead for PipeRead {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        loop {
            match Pin::new(&mut self.read).poll_read(cx, buf)? {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(0) => {
                    match Pin::new(&mut self.receiver).poll_next(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(None) => return Poll::Ready(Ok(0)),
                        Poll::Ready(Some(read)) => {
                            self.read = read;
                            continue;
                        }
                    }
                }
                Poll::Ready(n) => return Poll::Ready(Ok(n)),
            }
        }
    }
}

impl Write for PipeWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut noop_ctx = Context::from_waker(noop_waker_ref());
        match Pin::new(&mut self.write).poll_write(&mut noop_ctx, buf)? {
            Poll::Ready(x) => Ok(x),
            Poll::Pending => {
                let new_cap = (self.write.capacity() + 1).max(buf.len()).next_power_of_two();
                let (write, read) = bounded::pipe(new_cap);
                self.sender.try_send(read).unwrap();
                self.write = write;
                match Pin::new(&mut self.write).poll_write(&mut noop_ctx, buf)? {
                    Poll::Ready(x) => Ok(x),
                    Poll::Pending => panic!(),
                }
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

#[test]
fn test() {
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let (mut write, mut read) = pipe();
    let handle = spawner.spawn_with_handle(async move {
        let mut v = vec![0u8; 16];
        read.read_exact(v.as_mut_slice()).await.unwrap();
        assert_eq!(v, (0..=15).collect::<Vec<_>>());
        println!("Done reading");
    }).unwrap();
    let mut x = 0..=15;
    let mut i = 0;
    while !x.is_empty() {
        let vec = x.by_ref().take(i).collect::<Vec<_>>();
        write.write_all(&vec).unwrap();
        i += 1;
        pool.run_until_stalled();
    }
    handle.now_or_never().unwrap();
}