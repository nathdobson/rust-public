use std::io::Write;
use std::{mem, io};
use std::pin::Pin;

use pin_project::pin_project;
use std::task::{Poll, Context};
use tokio::io::AsyncRead;

#[pin_project]
pub struct Tokenizer<R: AsyncRead> {
    #[pin]
    inner: R,
    log: Vec<u8>,
}

impl<R: AsyncRead> Tokenizer<R> {
    pub fn new(inner: R) -> Self {
        Tokenizer {
            inner,
            log: vec![],
        }
    }
    pub fn clear_log(self: Pin<&mut Self>) {
        self.project().log.clear();
    }
    pub fn take_log(self: Pin<&mut Self>) -> Vec<u8> {
        mem::replace(&mut self.project().log, vec![])
    }
}

// impl<R: AsyncRead> AsyncRead for Tokenizer<R> {
//     fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
//         match self.as_mut().project().inner.poll_read(cx, buf)? {
//             Poll::Pending => Poll::Pending,
//             Poll::Ready(length) => {
//                 self.as_mut().project().log.write_all(&buf[..length]).unwrap();
//                 Poll::Ready(Ok(length))
//             }
//         }
//     }
// }

// impl<R: AsyncBufRead> AsyncBufRead for Tokenizer<R> {
//     fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
//         self.project().inner.poll_fill_buf(cx)
//     }
//
//     fn consume(self: Pin<&mut Self>, amt: usize) {
//         let mut this = self.project();
//         this.log.write(&this.inner.fill_buf().now_or_never().unwrap().unwrap()[..amt]).unwrap();
//         this.inner.consume(amt);
//     }
// }