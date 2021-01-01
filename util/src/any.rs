use std::any::{Any, type_name};
use std::{result, fmt, error};
use std::marker::Unsize;

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Error {
    from: &'static str,
    to: &'static str,
}

pub type Result<T> = result::Result<T, Error>;

pub trait Upcast<T: ?Sized> {
    fn upcast(&self) -> &T;
    fn upcast_mut(&mut self) -> &mut T;
    fn type_name(&self) -> &'static str;
}

impl<A: Unsize<B>, B: ?Sized> Upcast<B> for A {
    fn upcast(&self) -> &B { self }
    fn upcast_mut(&mut self) -> &mut B { self }
    fn type_name(&self) -> &'static str { type_name::<A>() }
}

pub trait AnyExt: Upcast<dyn Any> {
    fn downcast_ref_result<T: Any>(&self) -> Result<&T> {
        let from = (*self).type_name();
        let any = (*self).upcast();
        any.downcast_ref().ok_or(Error { from, to: type_name::<T>() })
    }

    fn downcast_mut_result<'a, T: Any>(&'a mut self) -> Result<&'a mut T> {
        let from = (*self).type_name();
        let any = (*self).upcast_mut();
        any.downcast_mut().ok_or(Error { from, to: type_name::<T>() })
    }
}

impl<T: Upcast<dyn Any> + ?Sized> AnyExt for T {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cannot cast from {:?} to {:?}", self.from, self.to)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl error::Error for Error {}

trait DynClone<U: ?Sized> {
    fn dyn_clone(&self) -> Box<U>;
}

impl<T: Clone + Unsize<U>, U:?Sized> DynClone<U> for T {
    fn dyn_clone(&self) -> Box<U> {
        Box::<T>::new(self.clone())
    }
}

#[test]
fn test_downcast() {
    trait Extension: Upcast<dyn Any> {
        fn foo(&self) {}
    }
    impl Extension for i32 {}
    impl Extension for u32 {}
    let mut x = 1u32;
    {
        let y: &dyn Extension = &x;
        assert_eq!(Ok(&1u32), y.downcast_ref_result());
        assert_eq!(Err(Error { from: "u32", to: "i32" }), y.downcast_ref_result::<i32>());
    }
    {
        let z: &mut dyn Extension = &mut x;
        assert_eq!(Ok(&mut 1u32), z.downcast_mut_result());
        assert_eq!(Err(Error { from: "u32", to: "i32" }), z.downcast_mut_result::<i32>());
    }
}

