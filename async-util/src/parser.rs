use futures::{AsyncRead, AsyncReadExt, FutureExt};
use std::io::Write;
use futures::task::{Context, Poll};
use pin_project::__private::Pin;
use std::{io};
use std::collections::{VecDeque};
use pin_project::pin_project;
use util::slice::SlicePair;
use futures::future::poll_fn;

#[pin_project]
pub struct Parser<R: AsyncRead + ?Sized> {
    freed: u64,
    front: u64,
    back: u64,
    buf: VecDeque<u8>,
    #[pin]
    inner: R,
}

impl<R: AsyncRead + ?Sized> Parser<R> {
    pub fn new(inner: R) -> Self where R: Sized {
        Parser {
            freed: 0,
            front: 0,
            back: 0,
            buf: VecDeque::new(),
            inner,
        }
    }
    pub fn free(self: Pin<&mut Self>, position: u64) {
        let this = self.project();
        assert!(position >= *this.freed);
        let diff = (position - *this.freed) as usize;
        assert!(diff <= this.buf.len());
        *this.freed = position;
        this.buf.drain(..diff);
    }
    pub fn seek_back(self: Pin<&mut Self>, position: u64) {
        let this = self.project();
        assert!(position >= *this.freed);
        assert!(position <= *this.freed + this.buf.len() as u64);
        *this.front = position;
    }
    pub fn position(self: Pin<&mut Self>) -> u64 {
        self.front
    }
    pub async fn lookahead<'a>(mut self: Pin<&'a mut Self>, lookahead: usize) -> io::Result<SlicePair<&'a [u8]>> {
        poll_fn(|cx| self.as_mut().poll_lookahead(cx, lookahead)).await?;
        Ok(self.as_slice())
    }
    pub fn as_slice<'a>(self: Pin<&'a mut Self>) -> SlicePair<&'a [u8]> {
        let this = self.project();
        SlicePair::from_deque(this.buf)
            .range((*this.front - *this.freed) as usize..(*this.back - *this.freed) as usize)
    }
    pub fn consume(self: Pin<&mut Self>, count: usize) {
        let this = self.project();
        assert!(*this.front + (count as u64) < *this.back);
        *this.front += count as u64;
    }
    pub fn poll_lookahead<'a>(mut self: Pin<&'a mut Self>, cx: &mut Context, lookahead: usize) -> Poll<io::Result<()>> {
        let mut this = self.as_mut().project();
        let stored = (*this.back - *this.freed) as usize;
        let batch_size: usize = 8196.max(this.buf.len()).max(lookahead);
        let min_size = stored + batch_size;
        if this.buf.len() < min_size {
            this.buf.resize(min_size, 0);
        }
        loop {
            if (*this.back - *this.front) as usize >= lookahead {
                return Poll::Ready(Ok(()));
            }
            let dest = SlicePair::from_deque_mut(&mut this.buf);
            let mut dest = dest.range(stored..min_size);
            match this.inner.as_mut().poll_read_vectored(cx, &mut dest.as_io_mut())? {
                Poll::Ready(count) => {
                    *this.back += count as u64;
                    if count == 0 {
                        return Poll::Ready(Ok(()));
                    }
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

impl<R: AsyncRead + ?Sized> AsyncRead for Parser<R> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        match self.as_mut().poll_lookahead(cx, 1)? {
            Poll::Ready(_) => {}
            Poll::Pending => return Poll::Pending,
        }
        let this = self.project();
        assert!(this.front <= this.back);
        let consumed = ((*this.back - *this.front) as usize).min(buf.len());
        let start = (*this.front - *this.freed) as usize;
        let source = SlicePair::from_deque(&this.buf).range(start..start + consumed);
        (&mut buf[..consumed]).write_vectored(&mut source.as_io()).unwrap();
        *this.front += consumed as u64;
        Poll::Ready(Ok(consumed))
    }
}

#[test]
fn test_slice_pair() {
    for n in 0..5 {
        let v1: Vec<i32> = (0..n as i32).collect();
        let mut v2 = vec![-1i32; n];
        for s1 in 0..n {
            for s2 in 0..n {
                let (v2a, v2b) = v2.split_at_mut(s2);
                SlicePair(v2a, v2b).copy_from(SlicePair(&v1[..s1], &v1[s1..]));
                assert_eq!(v1, v2);
            }
        }
    }
}

#[test]
fn test_parser() {
    use crate::pipe::unbounded::pipe;
    use futures::task::SpawnExt;
    use futures::executor::LocalPool;

    let (mut write, read) = pipe();
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    let joiner = spawner.spawn_with_handle(async {
        let mut parser = Box::pin(Parser::new(read));
        let mut parser = parser.as_mut();
        let mut buf = [0u8; 2];
        assert_eq!(2, parser.as_mut().read(&mut buf).await.unwrap());
        assert_eq!(buf, [1, 2]);
        parser.as_mut().free(1);
        assert_eq!(1, parser.as_mut().read(&mut buf).await.unwrap());
        assert_eq!(buf[..1], [3]);
        parser.as_mut().seek_back(1);
        assert_eq!(2, parser.as_mut().read(&mut buf).await.unwrap());
        assert_eq!(buf, [2, 3]);
    }).unwrap();
    pool.run_until_stalled();
    write.write(&[1, 2, 3]).unwrap();
    pool.run_until_stalled();
    let () = joiner.now_or_never().unwrap();
}