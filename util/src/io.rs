use std::io::{Write, IoSlice, BufWriter};
use std::{io, mem};
use std::fmt::Arguments;
use std::sync::{Mutex, Arc};
use crate::shared::Shared;
use crate::profile::Profile;

pub struct Pipeline {
    outgoing: Vec<Vec<u8>>,
}

pub struct PipelineWriter {
    buffer: Vec<u8>,
    shared: Arc<Mutex<Pipeline>>,
}

#[derive(Clone)]
pub struct PipelineFlusher {
    shared: Arc<Mutex<Pipeline>>,
}

//A non-blocking wrapper around a blocking Write:
//1. write to PipelineWriter (non-blocking)
//2. flush PipelineWriter (non-blocking)
//3. flush PipelineFlusher (blocking)
pub fn pipeline() -> (PipelineWriter, PipelineFlusher) {
    let shared1 = Arc::new(Mutex::new(Pipeline { outgoing: Vec::new() }));
    let shared2 = shared1.clone();
    (PipelineWriter { buffer: vec![], shared: shared1 },
     PipelineFlusher { shared: shared2 })
}

impl Write for PipelineWriter {
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
        if self.buffer.len() > 0 {
            self.shared.lock().unwrap().outgoing.push(
                mem::replace(&mut self.buffer, vec![]));
        }
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

impl PipelineFlusher {
    pub fn flush<W: Write>(&mut self, output: &mut W) -> io::Result<()> {
        let mut lock = self.shared.lock().unwrap();
        let writes = mem::replace(&mut lock.outgoing, vec![]);
        mem::drop(lock);
        for write in writes.iter() {
            println!("Flushing {}", write.len());
            output.write_all(write.as_slice())?
        }
        Ok(())
    }
}

impl PipelineFlusher {
    pub fn safe_flush<W: SafeWrite>(&mut self, output: &mut W) {
        self.flush(output).unwrap()
    }
}


#[macro_export]
macro_rules! swrite {
    ($dst:expr, $($arg:tt)*) => ($dst.safe_write_fmt(format_args!($($arg)*)))
}

pub trait SafeWrite: Write {
    fn safe_write_fmt(&mut self, args: Arguments) {
        Write::write_fmt(self, args).unwrap()
    }
    fn safe_write(&mut self, buf: &[u8]) -> usize {
        Write::write(self, buf).unwrap()
    }
    fn safe_write_all(&mut self, buf: &[u8]) {
        Write::write_all(self, buf).unwrap()
    }
    fn safe_flush(&mut self) {
        Write::flush(self).unwrap()
    }
}

impl SafeWrite for Vec<u8> {}

impl<'a, W: SafeWrite + ?Sized> SafeWrite for &'a mut W {}

impl<W: SafeWrite> SafeWrite for BufWriter<W> {}

impl<W: 'static> SafeWrite for Shared<W> where for<'a> &'a W: SafeWrite {}

impl<W: SafeWrite + ?Sized> SafeWrite for Box<W> {}

impl SafeWrite for PipelineWriter {}

pub struct ProfiledWrite<W: Write> {
    profile: Profile,
    inner: W,
}

impl<W: Write> Write for ProfiledWrite<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = self.inner.write(buf)?;
        self.profile.add(result);
        Ok(result)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()?;
        self.profile.flush();
        Ok(())
    }
}

impl<W: Write> ProfiledWrite<W> {
    pub fn new(inner: W, profile: Profile) -> Self {
        ProfiledWrite {
            profile,
            inner,
        }
    }
}

impl<W: SafeWrite> SafeWrite for ProfiledWrite<W> {}