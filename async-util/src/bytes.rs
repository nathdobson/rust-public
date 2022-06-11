use std::borrow::{Borrow, BorrowMut};
use std::future::Future;
use std::io;
use std::marker::PhantomData;
use std::mem::size_of;
use std::pin::Pin;

use futures::task::{Context, Poll};
use futures::{AsyncRead, AsyncWrite};
use pin_project::pin_project;

pub trait BytesInt {
    type Buffer: BorrowMut<[u8]> + Default + Copy;
    fn int_from_le(x: Self::Buffer) -> Self;
    fn int_from_be(x: Self::Buffer) -> Self;
    fn int_to_le(self) -> Self::Buffer;
    fn int_to_be(self) -> Self::Buffer;
}

impl BytesInt for u8 {
    type Buffer = [u8; 1];
    fn int_from_le(x: Self::Buffer) -> Self { Self::from_le_bytes(x) }
    fn int_from_be(x: Self::Buffer) -> Self { Self::from_be_bytes(x) }
    fn int_to_le(self) -> Self::Buffer { self.to_le_bytes() }
    fn int_to_be(self) -> Self::Buffer { self.to_be_bytes() }
}

impl BytesInt for u16 {
    type Buffer = [u8; 2];
    fn int_from_le(x: Self::Buffer) -> Self { Self::from_le_bytes(x) }
    fn int_from_be(x: Self::Buffer) -> Self { Self::from_be_bytes(x) }
    fn int_to_le(self) -> Self::Buffer { self.to_le_bytes() }
    fn int_to_be(self) -> Self::Buffer { self.to_be_bytes() }
}

impl BytesInt for u32 {
    type Buffer = [u8; 4];
    fn int_from_le(x: Self::Buffer) -> Self { Self::from_le_bytes(x) }
    fn int_from_be(x: Self::Buffer) -> Self { Self::from_be_bytes(x) }
    fn int_to_le(self) -> Self::Buffer { self.to_le_bytes() }
    fn int_to_be(self) -> Self::Buffer { self.to_be_bytes() }
}

impl BytesInt for u64 {
    type Buffer = [u8; 8];
    fn int_from_le(x: Self::Buffer) -> Self { Self::from_le_bytes(x) }
    fn int_from_be(x: Self::Buffer) -> Self { Self::from_be_bytes(x) }
    fn int_to_le(self) -> Self::Buffer { self.to_le_bytes() }
    fn int_to_be(self) -> Self::Buffer { self.to_be_bytes() }
}

impl BytesInt for u128 {
    type Buffer = [u8; 16];
    fn int_from_le(x: Self::Buffer) -> Self { Self::from_le_bytes(x) }
    fn int_from_be(x: Self::Buffer) -> Self { Self::from_be_bytes(x) }
    fn int_to_le(self) -> Self::Buffer { self.to_le_bytes() }
    fn int_to_be(self) -> Self::Buffer { self.to_be_bytes() }
}

impl BytesInt for usize {
    type Buffer = [u8; size_of::<usize>()];
    fn int_from_le(x: Self::Buffer) -> Self { Self::from_le_bytes(x) }
    fn int_from_be(x: Self::Buffer) -> Self { Self::from_be_bytes(x) }
    fn int_to_le(self) -> Self::Buffer { self.to_le_bytes() }
    fn int_to_be(self) -> Self::Buffer { self.to_be_bytes() }
}

pub trait Endian {
    fn int_from<T: BytesInt>(x: T::Buffer) -> T;
    fn int_to<T: BytesInt>(x: T) -> T::Buffer;
}

pub struct BE;

pub struct LE;

impl Endian for BE {
    fn int_from<T: BytesInt>(x: <T as BytesInt>::Buffer) -> T { T::int_from_be(x) }
    fn int_to<T: BytesInt>(x: T) -> <T as BytesInt>::Buffer { T::int_to_be(x) }
}

impl Endian for LE {
    fn int_from<T: BytesInt>(x: <T as BytesInt>::Buffer) -> T { T::int_from_le(x) }
    fn int_to<T: BytesInt>(x: T) -> <T as BytesInt>::Buffer { T::int_to_be(x) }
}

pub struct ReadInt<'a, R: AsyncRead + ?Sized, I: BytesInt, E: Endian> {
    read: &'a mut R,
    offset: usize,
    buf: I::Buffer,
    phantom: PhantomData<E>,
}

impl<'a, R: AsyncRead + ?Sized, I: BytesInt, E: Endian> Unpin for ReadInt<'a, R, I, E> {}

impl<'a, R: AsyncRead + ?Sized, I: BytesInt, E: Endian> ReadInt<'a, R, I, E> {
    fn new(read: &'a mut R) -> Self {
        ReadInt {
            read,
            offset: 0,
            buf: I::Buffer::default(),
            phantom: PhantomData,
        }
    }
}

pub trait ReadIntExt: AsyncRead + Unpin {
    fn read_le<I: BytesInt>(&mut self) -> ReadInt<Self, I, LE> { ReadInt::new(self) }
    fn read_be<I: BytesInt>(&mut self) -> ReadInt<Self, I, BE> { ReadInt::new(self) }
    fn read_int<I: BytesInt, E: Endian>(&mut self) -> ReadInt<Self, I, E> { ReadInt::new(self) }
    fn read_u8(&mut self) -> ReadInt<Self, u8, BE> { ReadInt::new(self) }
    fn read_u16_le(&mut self) -> ReadInt<Self, u16, LE> { ReadInt::new(self) }
    fn read_u32_le(&mut self) -> ReadInt<Self, u32, LE> { ReadInt::new(self) }
    fn read_u64_le(&mut self) -> ReadInt<Self, u64, LE> { ReadInt::new(self) }
    fn read_u16_be(&mut self) -> ReadInt<Self, u16, BE> { ReadInt::new(self) }
    fn read_u32_be(&mut self) -> ReadInt<Self, u32, BE> { ReadInt::new(self) }
    fn read_u64_be(&mut self) -> ReadInt<Self, u64, BE> { ReadInt::new(self) }
}

impl<T: ?Sized + AsyncRead + Unpin> ReadIntExt for T {}

impl<'a, R: ?Sized + AsyncRead + Unpin, I: BytesInt, E: Endian> Future for ReadInt<'a, R, I, E> {
    type Output = io::Result<I>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            let buf = &mut this.buf.borrow_mut()[this.offset..];
            if buf.is_empty() {
                return Poll::Ready(Ok(E::int_from(this.buf)));
            }
            match Pin::new(&mut *this.read).poll_read(cx, buf)? {
                Poll::Ready(r) => this.offset += r,
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[pin_project]
pub struct WriteBytes<'a, W: WriteIntExt + ?Sized, B: Borrow<[u8]>> {
    #[pin]
    write: &'a mut W,
    offset: usize,
    bytes: B,
}

pub trait WriteIntExt: AsyncWrite + Unpin {
    fn write_int<'a, I: BytesInt, E: Endian>(
        &'a mut self,
        x: I,
    ) -> WriteBytes<'a, Self, I::Buffer> {
        WriteBytes::new(self, E::int_to(x))
    }
    fn write_le<'a, I: BytesInt>(&'a mut self, x: I) -> WriteBytes<'a, Self, I::Buffer> {
        WriteBytes::new(self, I::int_to_le(x))
    }
    fn write_be<'a, I: BytesInt>(&'a mut self, x: I) -> WriteBytes<'a, Self, I::Buffer> {
        WriteBytes::new(self, I::int_to_be(x))
    }
    fn write_u8<'a>(&'a mut self, x: u8) -> WriteBytes<'a, Self, [u8; 1]> { self.write_le(x) }
    fn write_u16_le<'a>(&'a mut self, x: u16) -> WriteBytes<'a, Self, [u8; 2]> { self.write_le(x) }
    fn write_u16_be<'a>(&'a mut self, x: u16) -> WriteBytes<'a, Self, [u8; 2]> { self.write_be(x) }
    fn write_u32_le<'a>(&'a mut self, x: u32) -> WriteBytes<'a, Self, [u8; 4]> { self.write_le(x) }
    fn write_u32_be<'a>(&'a mut self, x: u32) -> WriteBytes<'a, Self, [u8; 4]> { self.write_be(x) }
    fn write_u64_le<'a>(&'a mut self, x: u64) -> WriteBytes<'a, Self, [u8; 8]> { self.write_le(x) }
    fn write_u64_be<'a>(&'a mut self, x: u64) -> WriteBytes<'a, Self, [u8; 8]> { self.write_be(x) }
}

impl<T: ?Sized + AsyncWrite + Unpin> WriteIntExt for T {}

impl<'a, W: WriteIntExt + ?Sized, B: Borrow<[u8]>> WriteBytes<'a, W, B> {
    fn new(write: &'a mut W, bytes: B) -> Self {
        WriteBytes {
            write,
            offset: 0,
            bytes,
        }
    }
}

impl<'a, W: WriteIntExt + ?Sized, B: Borrow<[u8]>> Future for WriteBytes<'a, W, B> {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut project = self.project();
        loop {
            let buf = &Borrow::<[u8]>::borrow(project.bytes)[*project.offset..];
            if buf.is_empty() {
                return Poll::Ready(Ok(()));
            }
            match project.write.as_mut().poll_write(cx, buf)? {
                Poll::Ready(x) => *project.offset += x,
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
