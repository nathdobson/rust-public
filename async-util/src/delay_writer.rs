use std::io::Write;
use std::collections::VecDeque;
use std::io;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;
use std::pin::Pin;
use util::slice::SlicePair;
use crate::poll::{PollResult, PollError};
use util::io::SafeWrite;
use crate::poll::PollResult::{Yield, Noop};

#[derive(Debug)]
pub struct DelayWriter(VecDeque<u8>);

impl SafeWrite for DelayWriter {}

impl Write for DelayWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let start = self.0.len();
        self.0.resize(start + buf.len(), 0);
        SlicePair::from_deque_mut(&mut self.0).range(start..start + buf.len()).copy_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}


impl DelayWriter {
    pub fn new() -> Self {
        DelayWriter(VecDeque::new())
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn poll_flush(&mut self, cx: &mut Context, mut output: Pin<&mut impl AsyncWrite>) -> PollResult<(), io::Error> {
        let buf = SlicePair::from_deque(&self.0);
        if buf.len() > 0 {
            match output.as_mut().poll_write_vectored(cx, &buf.as_io()).map_err(PollError::Abort)? {
                Poll::Ready(written) => {
                    self.0.drain(..written);
                    if !self.0.is_empty() {
                        return Yield(());
                    }
                }
                Poll::Pending => return Noop,
            }
        }
        output.poll_flush(cx).map_err(PollError::Abort)?.is_pending();
        Noop
    }
}