use std::sync::{Mutex, Arc};
use std::future::Future;
use std::task::{Context, Poll, Waker};
use std::pin::Pin;
use std::io;
use pin_project::pin_project;
use futures::pin_mut;
use crate::waker::test::TestWaker;
use futures::FutureExt;

enum State {
    Empty,
    Reading { dest: *mut [u8], waker: Option<Waker> },
    Writing { src: *const [u8], waker: Option<Waker> },
}

struct PipeRead(Arc<Mutex<State>>);

struct PipeWrite(Arc<Mutex<State>>);

struct ReadFuture<'a> {
    inner: &'a Mutex<State>,
    dest: &'a mut [u8],
}

struct WriteFuture<'a> {
    inner: &'a Mutex<State>,
    src: &'a [u8],
}

fn pipe() -> (PipeWrite, PipeRead) {
    let inner = Arc::new(Mutex::new(State::Empty));
    (PipeWrite(inner.clone()), PipeRead(inner.clone()))
}

impl PipeRead {
    fn read<'a>(&'a mut self, dest: &'a mut [u8]) -> ReadFuture {
        ReadFuture { inner: &*self.0, dest }
    }
}

impl PipeWrite {
    fn write<'a>(&'a mut self, src: &'a [u8]) -> WriteFuture {
        WriteFuture { inner: &*self.0, src }
    }
}

impl<'a> Future for ReadFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            let this = self.get_unchecked_mut();
            let mut lock = this.inner.lock().unwrap();
            match &mut *lock {
                State::Empty => {
                    *lock = State::Reading { dest: this.dest, waker: Some(cx.waker().clone()) };
                    Poll::Pending
                }
                State::Reading { dest, waker } => {
                    if dest.len() == this.dest.len() {
                        *waker = Some(cx.waker().clone());
                        Poll::Pending
                    } else {
                        let result = this.dest.len() - dest.len();
                        *lock = State::Empty;
                        Poll::Ready(Ok(result))
                    }
                }
                State::Writing { src, waker } => {
                    let count = src.len().min(this.dest.len());
                    this.dest[..count].copy_from_slice(&(&**src)[..count]);
                    waker.take().map(|x| x.wake());
                    *src = &(&**src as &[u8])[count..];
                    Poll::Ready(Ok(count))
                }
            }
        }
    }
}

impl<'a> Future for WriteFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            let this = self.get_unchecked_mut();
            let mut lock = this.inner.lock().unwrap();
            match &mut *lock {
                State::Empty => {
                    *lock = State::Writing { src: this.src, waker: Some(cx.waker().clone()) };
                    Poll::Pending
                }
                State::Reading { dest, waker } => {
                    let count = this.src.len().min(dest.len());
                    (&mut **dest)[..count].copy_from_slice(&this.src[..count]);
                    waker.take().map(|x| x.wake());
                    *dest = &mut (&mut **dest as &mut [u8])[count..];
                    Poll::Ready(Ok(count))
                }
                State::Writing { src, waker } => {
                    if src.len() == this.src.len() {
                        *waker = Some(cx.waker().clone());
                        Poll::Pending
                    } else {
                        let result = this.src.len() - src.len();
                        *lock = State::Empty;
                        Poll::Ready(Ok(result))
                    }
                }
            }
        }
    }
}


impl<'a> Drop for ReadFuture<'a> {
    fn drop(&mut self) {
        let mut lock = self.inner.lock().unwrap();
        match &*lock {
            State::Empty => {}
            State::Reading { dest, waker } => *lock = State::Empty,
            State::Writing { src, waker } => {}
        }
    }
}

impl<'a> Drop for WriteFuture<'a> {
    fn drop(&mut self) {
        let mut lock = self.inner.lock().unwrap();
        match &*lock {
            State::Empty => {}
            State::Reading { dest, waker } => {}
            State::Writing { src, waker } => *lock = State::Empty,
        }
    }
}

#[test]
fn test_simple() {
    let (mut write, mut read) = pipe();
    let writer = async {
        assert_eq!(write.write(&[0, 1]).await.unwrap(), 2);
        assert_eq!(write.write(&[2, 3, 4, 5]).await.unwrap(), 2);
    }.fuse();
    let reader = async {
        let mut x = [0; 4];
        assert_eq!(read.read(&mut x).await.unwrap(), 2);
        assert_eq!(&x[..2], &[0, 1]);
        assert_eq!(read.read(&mut x[..2]).await.unwrap(), 2);
        assert_eq!(&x[..2], &[2, 3]);
    }.fuse();
    pin_mut!(writer, reader);
    let (test1, waker1) = TestWaker::new();
}
