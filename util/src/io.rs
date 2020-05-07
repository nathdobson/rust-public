use std::io::{Write, IoSlice};
use std::io;
use std::fmt::Arguments;

pub struct FullBufWriter<W: Write> {
    buffer: Vec<u8>,
    inner: W,
}

impl<W: Write> FullBufWriter<W> {
    pub fn new(inner: W) -> Self {
        FullBufWriter {
            buffer: vec![],
            inner,
        }
    }
}

impl<W: Write> Write for FullBufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.write(buf)
    }
    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize> {
        self.buffer.write_vectored(bufs)
    }
    fn is_write_vectored(&self) -> bool {
        self.buffer.is_write_vectored()
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.write_all(&self.buffer)?;
        self.buffer.clear();
        self.inner.flush()?;
        Ok(())
    }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.buffer.write_all(buf)
    }
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice]) -> io::Result<()> {
        self.buffer.write_all_vectored(bufs)
    }
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> {
        self.buffer.write_fmt(fmt)
    }
}

