use std::{io, mem};
use std::io::{BufRead, Read, Write};

pub struct Tokenizer<R: Read> {
    inner: R,
    log: Vec<u8>,
}

impl<R: Read> Tokenizer<R> {
    pub fn new(inner: R) -> Self {
        Tokenizer {
            inner,
            log: vec![],
        }
    }
    pub fn clear_log(&mut self) {
        self.log.clear();
    }
    pub fn take_log(&mut self) -> Vec<u8> {
        mem::replace(&mut self.log, vec![])
    }
}

impl<R: Read> Read for Tokenizer<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.inner.read(buf)?;
        self.log.write_all(&buf[..len])?;
        Ok(len)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner.read_exact(buf)?;
        self.log.write_all(buf)?;
        Ok(())
    }
}

impl<R: BufRead> BufRead for Tokenizer<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.log.write(&self.inner.fill_buf().unwrap()[0..amt]).unwrap();
        self.inner.consume(amt);
    }
}