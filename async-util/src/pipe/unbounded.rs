use std::sync::atomic::AtomicPtr;
use crate::waker::AtomicWaker;
use crate::pipe::bounded;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::io;
use std::io::{Error, Write};
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf, AsyncReadExt};
use std::stream::Stream;
use crate::waker::noop_waker_ref;
use tokio::task::yield_now;
use crate::futureext::FutureExt;

pub struct PipeWrite {
    write: bounded::PipeWrite,
    sender: UnboundedSender<bounded::PipeRead>,
}

pub struct PipeRead {
    read: bounded::PipeRead,
    receiver: UnboundedReceiver<bounded::PipeRead>,
}

pub fn pipe() -> (PipeWrite, PipeRead) {
    let (sender, receiver) = unbounded_channel();
    let (write, read) = bounded::pipe(0);
    (PipeWrite { write, sender }, PipeRead { read, receiver })
}

impl Unpin for PipeWrite {}

impl Unpin for PipeRead {}

impl AsyncRead for PipeRead {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf) -> Poll<io::Result<()>> {
        loop {
            let old_filled = buf.filled().len();
            match Pin::new(&mut self.read).poll_read(cx, buf)? {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(()) if buf.filled().len() == old_filled => {
                    match Pin::new(&mut self.receiver).poll_recv(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(None) => return Poll::Ready(Ok(())),
                        Poll::Ready(Some(read)) => {
                            self.read = read;
                            continue;
                        }
                    }
                }
                Poll::Ready(()) => return Poll::Ready(Ok(())),
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
                self.sender.send(read).ok();
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

#[tokio::test]
async fn test() {
    let (mut write, mut read) = pipe();
    let handle = tokio::spawn(async move {
        let mut v = vec![0u8; 16];
        read.read_exact(v.as_mut_slice()).await.unwrap();
        assert_eq!(v, (0..=15).collect::<Vec<_>>());
        println!("Done reading");
    });
    let mut x = 0..=15;
    let mut i = 0;
    while !x.is_empty() {
        let vec = x.by_ref().take(i).collect::<Vec<_>>();
        write.write_all(&vec).unwrap();
        i += 1;
        yield_now().await;
    }
    handle.ready().unwrap().unwrap();
}