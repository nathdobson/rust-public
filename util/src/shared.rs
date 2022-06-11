#![allow(deprecated)]

use std::any::Any;
use std::cmp::Ordering;
use std::fmt::{Arguments, Formatter};
use std::hash::Hasher;
use std::io::{IoSlice, IoSliceMut, Read, Write};
use std::marker::Unsize;
use std::ops::{CoerceUnsized, Deref};
use std::pin::Pin;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak};
use std::{fmt, hash, io, mem};

use futures::task::{Context, Poll};
use futures::{AsyncRead, AsyncWrite};

pub struct Shared<T: ?Sized>(Arc<T>);

pub struct WkShared<T: ?Sized>(Weak<T>);

pub struct SharedMut<T: ?Sized>(Shared<RwLock<T>>);

pub struct WkSharedMut<T: ?Sized>(WkShared<RwLock<T>>);

impl<T: ?Sized> Shared<T> {
    pub fn new(x: T) -> Self
    where
        T: Sized,
    {
        Shared(Arc::new(x))
    }
    pub fn into_inner(self) -> Result<T, Self>
    where
        T: Sized,
    {
        Ok(Arc::try_unwrap(self.0).map_err(Shared)?)
    }
    pub fn downgrade(&self) -> WkShared<T> { WkShared(Arc::downgrade(&self.0)) }
    pub fn as_ptr(&self) -> *const u8 { unsafe { mem::transmute_copy::<Self, *const u8>(self) } }
}

impl<T: ?Sized> WkShared<T> {
    pub fn upgrade(&self) -> Option<Shared<T>> { self.0.upgrade().map(Shared) }
    pub fn as_ptr(&self) -> *const u8 { unsafe { mem::transmute_copy::<Self, *const u8>(self) } }
}

impl<T: ?Sized> SharedMut<T> {
    pub fn new(x: T) -> Self
    where
        T: Sized,
    {
        SharedMut(Shared::new(RwLock::new(x)))
    }
    pub fn into_inner(self) -> Result<T, Self>
    where
        T: Sized,
    {
        let result = self.0.into_inner().map_err(SharedMut)?;
        Ok(result.into_inner().unwrap())
    }
    pub fn borrow(&self) -> RwLockReadGuard<T> { self.0.try_read().unwrap() }
    pub fn borrow_mut(&self) -> RwLockWriteGuard<T> { self.0.try_write().unwrap() }
    pub fn downgrade(&self) -> WkSharedMut<T> { WkSharedMut(self.0.downgrade()) }
    pub fn as_ptr(&self) -> *const u8 { unsafe { mem::transmute_copy::<Self, *const u8>(self) } }
}

impl<T: ?Sized> WkSharedMut<T> {
    pub fn upgrade(&self) -> Option<SharedMut<T>> { self.0.upgrade().map(SharedMut) }
    pub fn as_ptr(&self) -> *const u8 { unsafe { mem::transmute_copy::<Self, *const u8>(self) } }
}

impl SharedMut<dyn Any + 'static + Send + Sync> {
    pub fn downcast<T: Any>(self) -> Result<SharedMut<T>, Self> {
        unsafe {
            if self.borrow().is::<T>() {
                let raw: raw::TraitObject = mem::transmute(self);
                Ok(mem::transmute(raw.data))
            } else {
                Err(self)
            }
        }
    }
}

impl Shared<dyn Any + 'static + Sync + Send> {
    pub fn downcast<T: Any + Sync + Send>(self) -> Result<Shared<T>, Self> {
        Ok(Shared(Arc::downcast(self.0).map_err(Shared)?))
    }
}

impl<T: ?Sized> Eq for Shared<T> {}

impl<T: ?Sized> Eq for WkShared<T> {}

impl<T: ?Sized> Eq for SharedMut<T> {}

impl<T: ?Sized> Eq for WkSharedMut<T> {}

impl<T: ?Sized> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool { self.as_ptr().eq(&other.as_ptr()) }
}

impl<T: ?Sized> PartialEq for SharedMut<T> {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}

impl<T: ?Sized> PartialEq for WkShared<T> {
    fn eq(&self, other: &Self) -> bool { self.as_ptr().eq(&other.as_ptr()) }
}

impl<T: ?Sized> PartialEq for WkSharedMut<T> {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}

impl<T: ?Sized> PartialOrd for Shared<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

impl<T: ?Sized> PartialOrd for SharedMut<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { self.0.partial_cmp(&other.0) }
}

impl<T: ?Sized> PartialOrd for WkShared<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

impl<T: ?Sized> PartialOrd for WkSharedMut<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { self.0.partial_cmp(&other.0) }
}

impl<T: ?Sized> Ord for Shared<T> {
    fn cmp(&self, other: &Self) -> Ordering { self.as_ptr().cmp(&other.as_ptr()) }
}

impl<T: ?Sized> Ord for SharedMut<T> {
    fn cmp(&self, other: &Self) -> Ordering { self.0.cmp(&other.0) }
}

impl<T: ?Sized> Ord for WkShared<T> {
    fn cmp(&self, other: &Self) -> Ordering { self.as_ptr().cmp(&other.as_ptr()) }
}

impl<T: ?Sized> Ord for WkSharedMut<T> {
    fn cmp(&self, other: &Self) -> Ordering { self.0.cmp(&other.0) }
}

impl<T: ?Sized> hash::Hash for Shared<T> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.as_ptr().hash(state) }
}

impl<T: ?Sized> hash::Hash for SharedMut<T> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state) }
}

impl<T: ?Sized> hash::Hash for WkShared<T> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.as_ptr().hash(state) }
}

impl<T: ?Sized> hash::Hash for WkSharedMut<T> {
    fn hash<H: Hasher>(&self, state: &mut H) { self.0.hash(state) }
}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self { Shared(self.0.clone()) }
}

impl<T: ?Sized> Clone for SharedMut<T> {
    fn clone(&self) -> Self { SharedMut(self.0.clone()) }
}

impl<T: ?Sized> Clone for WkShared<T> {
    fn clone(&self) -> Self { WkShared(self.0.clone()) }
}

impl<T: ?Sized> Clone for WkSharedMut<T> {
    fn clone(&self) -> Self { WkSharedMut(self.0.clone()) }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} -> ", self.as_ptr())?;
        self.0.fmt(f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for SharedMut<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} -> ", self.as_ptr())?;
        self.0.fmt(f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for WkShared<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { write!(f, "{:?}", self.as_ptr()) }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for WkSharedMut<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { write!(f, "{:?}", self.as_ptr()) }
}

impl<T: ?Sized> Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { self.0.deref() }
}

impl<T, U> CoerceUnsized<Shared<U>> for Shared<T>
where
    T: ?Sized + Unsize<U>,
    U: ?Sized,
{
}

impl<T, U> CoerceUnsized<SharedMut<U>> for SharedMut<T>
where
    T: ?Sized + Unsize<U>,
    U: ?Sized,
{
}

impl<'a, T: ?Sized> Write for &'a Shared<T>
where
    &'a T: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.0.deref().write(buf) }
    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize> {
        self.0.deref().write_vectored(bufs)
    }
    fn flush(&mut self) -> io::Result<()> { self.0.deref().flush() }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> { self.0.deref().write_all(buf) }
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice]) -> io::Result<()> {
        self.0.deref().write_all_vectored(bufs)
    }
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> { self.0.deref().write_fmt(fmt) }
}

impl<'a, T: ?Sized> Read for &'a Shared<T>
where
    &'a T: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.0.deref().read(buf) }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        self.0.deref().read_vectored(bufs)
    }
    fn is_read_vectored(&self) -> bool { self.0.deref().is_read_vectored() }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.0.deref().read_to_end(buf)
    }
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.0.deref().read_to_string(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> { self.0.deref().read_exact(buf) }
}

impl<T: ?Sized> Read for Shared<T>
where
    for<'a> &'a T: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.0.deref().read(buf) }
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        self.0.deref().read_vectored(bufs)
    }
    fn is_read_vectored(&self) -> bool { self.0.deref().is_read_vectored() }
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.0.deref().read_to_end(buf)
    }
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.0.deref().read_to_string(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> { self.0.deref().read_exact(buf) }
}

impl<T: ?Sized + 'static> Write for Shared<T>
where
    for<'a> &'a T: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.0.deref().write(buf) }
    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize> {
        self.0.deref().write_vectored(bufs)
    }
    fn flush(&mut self) -> io::Result<()> { self.0.deref().flush() }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> { self.0.deref().write_all(buf) }
    fn write_all_vectored(&mut self, bufs: &mut [IoSlice]) -> io::Result<()> {
        self.0.deref().write_all_vectored(bufs)
    }
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> { self.0.deref().write_fmt(fmt) }
}

pub trait ObjectInner: Any + fmt::Debug + 'static + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl<T> ObjectInner for T
where
    T: fmt::Debug + 'static + Send + Sync,
{
    fn as_any(&self) -> &dyn Any { self }
}

pub type Object = WkShared<dyn ObjectInner>;

impl<T: ObjectInner> Shared<T> {
    pub fn as_object(&self) -> Object {
        let result: Shared<dyn ObjectInner> = self.clone();
        result.downgrade()
    }
}

impl<T: ?Sized> AsyncRead for Shared<T>
where
    for<'a> &'a T: AsyncRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &**self).poll_read(cx, buf)
    }
}

impl<'a, T: ?Sized> AsyncRead for &'a Shared<T>
where
    for<'b> &'b T: AsyncRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &**self).poll_read(cx, buf)
    }
}

impl<T: ?Sized> AsyncWrite for Shared<T>
where
    for<'a> &'a T: AsyncWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &**self).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &**self).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &**self).poll_close(cx)
    }
}

impl<'a, T: ?Sized> AsyncWrite for &'a Shared<T>
where
    for<'b> &'b T: AsyncWrite,
{
    fn poll_write<'b>(
        self: Pin<&'b mut Self>,
        cx: &'b mut Context<'_>,
        buf: &'b [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &**self).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &**self).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &**self).poll_close(cx)
    }
}

//#[derive(Debug)]
//pub struct Header<T: HasHeader<H>, H> {
//    this: Option<WkSharedMut<T>>,
//    header: H,
//}
//
//impl<T: HasHeader<H>, H> Header<T, H> {
//    pub fn new_header(header: H) -> Self {
//        Header {
//            this: None,
//            header,
//        }
//    }
//    pub fn new_shared(value: T) -> SharedMut<T> {
//        let result = SharedMut::new(value);
//        result.borrow_mut().shared_header_mut().this = Some(result.downgrade());
//        result
//    }
//}
//
//pub trait HasHeader<H>: Sized {
//    fn shared_header(&self) -> &Header<Self, H>;
//    fn shared_header_mut(&mut self) -> &mut Header<Self, H>;
//    fn this(&self) -> SharedMut<Self> {
//        self.shared_header().this.as_ref().unwrap().upgrade().unwrap()
//    }
//}
//
//pub trait HasHeaderExt<H> {
//    fn header(&self) -> &H;
//    fn header_mut(&mut self) -> &mut H;
//}
//
//impl<H, T> HasHeaderExt<H> for T where T: HasHeader<H> {
//    fn header(&self) -> &H {
//        &self.shared_header().header
//    }
//
//    fn header_mut(&mut self) -> &mut H {
//        &mut self.shared_header_mut().header
//    }
//}

#[test]
fn test_ptr() {
    let a = Shared::new(1);
    let b = a.clone();
    let c = a.downgrade();
    let d = b.downgrade();
    assert_eq!(a.as_ptr(), b.as_ptr());
    assert_eq!(a.as_ptr(), c.as_ptr());
    assert_eq!(a.as_ptr(), d.as_ptr());
}
//
//#[test]
//fn test_shared() {
//    #[derive(Debug)]
//    struct Foo {
//        header: Header<Foo, usize>,
//        footer: usize,
//    }
//    impl HasHeader<usize> for Foo {
//        fn shared_header(&self) -> &Header<Self, usize> { &self.header }
//        fn shared_header_mut(&mut self) -> &mut Header<Self, usize> { &mut self.header }
//    }
//    impl Foo {
//        fn new(x: usize) -> SharedMut<Foo> {
//            Header::new_shared(Foo { header: Header::new_header(x + 10), footer: x + 20 })
//        }
//    }
//    let foo = Foo::new(0);
//    assert_eq!(foo.borrow().this(), foo);
//    assert_eq!(foo.borrow().header(), &10);
//    *foo.borrow_mut().header_mut() += 1;
//    assert_eq!(foo.borrow().header(), &11);
//    assert_eq!(foo.borrow().footer, 20);
//}
