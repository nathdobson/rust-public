use std::io::{Write, BufWriter};
use std::fmt::Arguments;
use util::shared::Shared;

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
    fn safe_flush(&mut self) {
        Write::flush(self).unwrap()
    }
}

impl SafeWrite for Vec<u8> {}

impl<'a, W: SafeWrite + ?Sized> SafeWrite for &'a mut W {}

impl<W: SafeWrite> SafeWrite for BufWriter<W> {}

impl<T:'static> SafeWrite for Shared<T> where for<'a> &'a T: SafeWrite {}