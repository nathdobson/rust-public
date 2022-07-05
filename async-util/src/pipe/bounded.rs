use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::{io, mem};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::task::yield_now;
use tokio::{join, try_join};
use util::slice::{raw_split_at, raw_split_at_mut, vec_as_slice_raw, Slice, SlicePair};
use waker_util::AtomicWaker;

struct Inner {
    memory: Vec<u8>,
    length: AtomicUsize,
    closed: AtomicBool,
    reader: AtomicWaker,
    writer: AtomicWaker,
}

pub struct PipeWrite {
    inner: Arc<Inner>,
    write_head: usize,
}

pub struct PipeRead {
    inner: Arc<Inner>,
    read_head: usize,
}

pub fn pipe(capacity: usize) -> (PipeWrite, PipeRead) {
    let inner = Arc::new(Inner {
        memory: vec![0u8; capacity],
        length: AtomicUsize::new(0),
        closed: AtomicBool::new(false),
        reader: AtomicWaker::new(),
        writer: AtomicWaker::new(),
    });
    (
        PipeWrite {
            inner: inner.clone(),
            write_head: 0,
        },
        PipeRead {
            inner,
            read_head: 0,
        },
    )
}

impl Unpin for PipeWrite {}

impl Unpin for PipeRead {}

impl PipeRead {
    pub fn capacity(&self) -> usize { self.inner.memory.len() }
}

impl PipeWrite {
    pub fn capacity(&self) -> usize { self.inner.memory.len() }
}

impl AsyncRead for PipeRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        unsafe {
            let mut length = self.inner.length.load(Acquire);
            if length == 0 {
                if self.inner.closed.load(Relaxed) {
                    return Poll::Ready(Ok(()));
                }
                self.inner.reader.register(cx.waker());
                length = self.inner.length.load(Acquire);
                if length == 0 {
                    if self.inner.closed.load(Relaxed) {
                        return Poll::Ready(Ok(()));
                    }
                    return Poll::Pending;
                }
            }
            length = length.min(buf.remaining());
            let slice = vec_as_slice_raw(&self.inner.memory);
            let SlicePair(second, first) = raw_split_at(slice, self.read_head);
            let SlicePair(first, second) = SlicePair(first, second).range_unsafe(..length).as_ref();
            buf.put_slice(first);
            buf.put_slice(second);
            self.read_head = (self.read_head + length) % self.inner.memory.len();
            self.inner.length.fetch_sub(length, Release);
            self.inner.writer.wake();
            Poll::Ready(Ok(()))
        }
    }
}

impl AsyncWrite for PipeWrite {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        unsafe {
            let mut length = self.inner.length.load(Acquire);
            if length == self.inner.memory.len() {
                self.inner.writer.register(cx.waker());
                length = self.inner.length.load(Acquire);
                if length == self.inner.memory.len() {
                    return Poll::Pending;
                }
            }
            let written = buf.len().min(self.inner.memory.len() - length);
            let slice = vec_as_slice_raw(&self.inner.memory);
            let SlicePair(second, first) = raw_split_at_mut(slice as *mut [u8], self.write_head);
            let dest = SlicePair(first, second).range_unsafe(..written);
            dest.as_mut().copy_from_slice(&buf[..written]);
            self.write_head = (self.write_head + written) % self.inner.memory.len();
            self.inner.length.fetch_add(written, Release);
            self.inner.reader.wake();
            Poll::Ready(Ok(written))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.inner.closed.store(true, Relaxed);
        self.inner.reader.wake();
        Poll::Ready(Ok(()))
    }
}

impl Drop for PipeWrite {
    fn drop(&mut self) {
        self.inner.closed.store(true, Relaxed);
        self.inner.reader.wake();
    }
}

#[tokio::test]
async fn test() {
    let (mut write, mut read) = pipe(4);
    let writer = async {
        let mut x = 0..=15;
        let mut i = 0;
        while !x.is_empty() {
            let vec = x.by_ref().take(i).collect::<Vec<_>>();
            write.write_all(&vec).await.unwrap();
            i += 1;
        }
        println!("Done writing");
    };
    let reader = async {
        let mut v = vec![0u8; 16];
        read.read_exact(v.as_mut_slice()).await.unwrap();
        assert_eq!(v, (0..=15).collect::<Vec<_>>());
        println!("Done reading");
    };
    join!(writer, reader);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parallel() {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    let (mut write, mut read) = pipe(64);
    let expected = Arc::new(
        thread_rng()
            .sample_iter(Alphanumeric)
            .take(100000)
            .collect::<Vec<_>>(),
    );
    let writer = tokio::spawn({
        let expected = expected.clone();
        async move {
            let mut iter = expected.iter();
            loop {
                let buf = iter
                    .by_ref()
                    .take(thread_rng().gen_range(1..256))
                    .cloned()
                    .collect::<Vec<_>>();
                if buf.is_empty() {
                    break;
                }
                write.write_all(&buf).await.unwrap();
            }
        }
    });
    let reader = tokio::spawn({
        let expected = expected.clone();
        let mut actual = vec![];
        async move {
            loop {
                let length = thread_rng().gen_range(1..256);
                let offset = actual.len();
                actual.resize(offset + length, 0);
                let count = read.read(&mut actual[offset..]).await.unwrap();
                actual.truncate(offset + count);
                if count == 0 {
                    break;
                }
            }
            assert_eq!(actual, *expected);
        }
    });
    try_join!(writer, reader).unwrap();
}
