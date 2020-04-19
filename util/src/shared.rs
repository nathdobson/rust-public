use std::sync::{Arc, Weak, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::cmp::Ordering;
use std::{hash, fmt, mem};
use std::hash::Hasher;
use std::fmt::Formatter;
use std::marker::Unsize;
use std::ops::CoerceUnsized;


// A ref counted pointer with interior mutable and identity of the memory address itself.
pub struct Shared<T: ?Sized>(Arc<RwLock<T>>);

pub struct WkShared<T: ?Sized>(Weak<RwLock<T>>);

impl<T: ?Sized> Shared<T> {
    pub fn new(x: T) -> Shared<T> where T: Sized {
        Shared(Arc::new(RwLock::new(x)))
    }
    pub fn into_inner(self) -> Result<T, Self> where T: Sized {
        let inner = Arc::try_unwrap(self.0).map_err(Shared)?;
        Ok(inner.into_inner().unwrap())
    }
    pub fn borrow(&self) -> RwLockReadGuard<T> {
        self.0.try_read().unwrap()
    }
    pub fn borrow_mut(&self) -> RwLockWriteGuard<T> {
        self.0.try_write().unwrap()
    }
    pub fn downgrade(&self) -> WkShared<T> {
        WkShared(Arc::downgrade(&self.0))
    }
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { mem::transmute_copy::<Self, *const u8>(self) }
    }
}

impl<T: ?Sized> WkShared<T> {
    pub fn upgrade(&self) -> Option<Shared<T>> {
        self.0.upgrade().map(Shared)
    }
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { mem::transmute_copy::<Self, *const u8>(self) }
    }
}

impl<T: ?Sized> Eq for Shared<T> {}

impl<T: ?Sized> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr().eq(&other.as_ptr())
    }
}

impl<T: ?Sized> PartialOrd for Shared<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

impl<T: ?Sized> Ord for Shared<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

impl<T: ?Sized> hash::Hash for Shared<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ((&*self.0) as *const RwLock<T>).hash(state)
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} -> ", self.as_ptr())?;
        self.borrow().fmt(f)
    }
}


impl<T: ?Sized + fmt::Debug> fmt::Debug for WkShared<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_ptr())
    }
}

impl<T, U> CoerceUnsized<Shared<U>> for Shared<T> where T: ?Sized + Unsize<U>, U: ?Sized {}

#[derive(Debug)]
pub struct Header<T: HasHeader<H>, H> {
    this: Option<WkShared<T>>,
    header: H,
}

impl<T: HasHeader<H>, H> Header<T, H> {
    pub fn new_header(header: H) -> Self {
        Header {
            this: None,
            header,
        }
    }
    pub fn new_shared(value: T) -> Shared<T> {
        let result = Shared::new(value);
        result.borrow_mut().shared_header_mut().this = Some(result.downgrade());
        result
    }
}

pub trait HasHeader<H>: Sized {
    fn shared_header(&self) -> &Header<Self, H>;
    fn shared_header_mut(&mut self) -> &mut Header<Self, H>;
    fn this(&self) -> Shared<Self> {
        self.shared_header().this.as_ref().unwrap().upgrade().unwrap()
    }
}

pub trait HasHeaderExt<H> {
    fn header(&self) -> &H;
    fn header_mut(&mut self) -> &mut H;
}

impl<H, T> HasHeaderExt<H> for T where T: HasHeader<H> {
    fn header(&self) -> &H {
        &self.shared_header().header
    }

    fn header_mut(&mut self) -> &mut H {
        &mut self.shared_header_mut().header
    }
}

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

#[test]
fn test_shared() {
    #[derive(Debug)]
    struct Foo {
        header: Header<Foo, usize>,
        footer: usize,
    }
    impl HasHeader<usize> for Foo {
        fn shared_header(&self) -> &Header<Self, usize> { &self.header }
        fn shared_header_mut(&mut self) -> &mut Header<Self, usize> { &mut self.header }
    }
    impl Foo {
        fn new(x: usize) -> Shared<Foo> {
            Header::new_shared(Foo { header: Header::new_header(x + 10), footer: x + 20 })
        }
    }
    let foo = Foo::new(0);
    assert_eq!(foo.borrow().this(), foo);
    assert_eq!(foo.borrow().header(), &10);
    *foo.borrow_mut().header_mut() += 1;
    assert_eq!(foo.borrow().header(), &11);
    assert_eq!(foo.borrow().footer, 20);
}