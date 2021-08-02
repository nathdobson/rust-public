#![allow(deprecated)]

use std::any::{Any, type_name, TypeId};
use std::{result, fmt, error, mem, sync, rc};
use std::marker::{Unsize, PhantomData};
use std::ops::{CoerceUnsized, Deref};
use std::ptr::{null, null_mut};
use std::raw::TraitObject;
use std::sync::Arc;
use std::rc::Rc;

pub struct Error<T> {
    pub unused: T,
    pub from: &'static str,
    pub to: &'static str,
}

impl<T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cannot cast from `{}' to `{}'", self.from, self.to)
    }
}

impl<T> fmt::Debug for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<T> error::Error for Error<T> {}

struct Vtable<T: ?Sized>(*mut (), PhantomData<T>);

impl<T: ?Sized> Vtable<T> {
    unsafe fn new(x: *mut ()) -> Self { Vtable(x, PhantomData) }
    fn null(&self) -> *const T {
        unsafe {
            mem::transmute_copy(&TraitObject { data: null_mut(), vtable: self.0 })
        }
    }
    unsafe fn unwrap<P: Pointy<Target=T>>(x: &P) -> (*mut (), Self) {
        assert_eq!(mem::size_of::<P>(), mem::size_of::<TraitObject>());
        let object: TraitObject = mem::transmute_copy(x);
        (object.data, Vtable::new(object.vtable))
    }
    fn type_info(&self) -> TypeInfo where T: RawAny {
        RawAny::raw_type_info(self.null())
    }
}

impl<T: ?Sized> Clone for Vtable<T> {
    fn clone(&self) -> Self {
        Vtable(self.0, PhantomData)
    }
}

impl<T: ?Sized> Copy for Vtable<T> {}

pub struct TypeInfo { id: TypeId, name: &'static str }

impl TypeInfo {
    pub fn of<T: 'static + ?Sized>() -> Self { TypeInfo { id: TypeId::of::<T>(), name: type_name::<T>() } }
}

pub unsafe trait RawAny: 'static {
    fn raw_type_info(self: *const Self) -> TypeInfo;
}

unsafe impl<T: 'static + Sized> RawAny for T {
    fn raw_type_info(self: *const Self) -> TypeInfo { TypeInfo::of::<T>() }
}

pub unsafe trait Pointy: Sized {
    type Target: ?Sized;
}

trait Downcast1<T>: Sized {
    fn downcast1(self) -> Result<T, Error<Self>>;
}

impl<T, U> Downcast1<T> for U {
    default fn downcast1(self) -> Result<T, Error<Self>> { return Err(Error { unused: self, from: "?", to: "?" }); }
}

pub trait TypeEquals {
    type Other: ?Sized;
}

impl<T: ?Sized> TypeEquals for T {
    type Other = Self;
}

impl<T, U> Downcast1<T> for U where T: TypeEquals<Other=U> {
    fn downcast1(self) -> Result<T, Error<Self>> {
        unsafe {
            let result = mem::transmute_copy(&self);
            mem::forget(self);
            Ok(result)
        }
    }
}

pub trait Downcast<T>: Sized {
    fn downcast(self) -> Result<T, Error<Self>>;
}

impl<T, U> Downcast<T> for U {
    default fn downcast(self) -> Result<T, Error<Self>> {
        Downcast1::downcast1(self)
    }
}

impl<T, U> Downcast<T> for U
    where T: CoerceUnsized<U> + Pointy,
          T::Target: Sized + 'static,
          U: Pointy,
          U::Target: RawAny {
    fn downcast(self) -> Result<T, Error<Self>> { downcast_sized(self) }
}

pub fn downcast_sized<U, T>(this: U) -> Result<T, Error<U>>
    where T: CoerceUnsized<U> + Pointy,
          T::Target: Sized + 'static,
          U: Pointy,
          U::Target: RawAny {
    unsafe {
        assert_eq!(mem::size_of::<T>(), mem::size_of::<*mut ()>());
        let (data, actual_vtable): (*mut (), Vtable<U::Target>) = Vtable::unwrap(&this);
        let actual_type = actual_vtable.type_info();
        let expected_type = TypeInfo::of::<T::Target>();
        if actual_type.id == expected_type.id {
            mem::forget(this);
            Ok(mem::transmute_copy::<_, T>(&data))
        } else {
            Err(Error { unused: this, from: actual_type.name, to: expected_type.name })
        }
    }
}

pub trait Upcast<U> {
    fn upcast(self) -> U;
}

trait Upcast1<U> {
    fn upcast1(self) -> U;
}

impl<T, U> Upcast1<U> for T {
    default fn upcast1(self) -> U { panic!("Bad upcast"); }
}

impl<T, U> Upcast1<U> for T where U: TypeEquals<Other=T> {
    default fn upcast1(self) -> U {
        unsafe {
            let result = mem::transmute_copy(&self);
            mem::forget(self);
            result
        }
    }
}

impl<T, U> Upcast<U> for T {
    default fn upcast(self) -> U { Upcast1::upcast1(self) }
}

impl<T, U> Upcast<U> for T where T: CoerceUnsized<U> {
    fn upcast(self) -> U { self }
}

unsafe impl<'a, T: ?Sized> Pointy for &'a T { type Target = T; }

unsafe impl<'a, T: ?Sized> Pointy for &'a mut T { type Target = T; }

unsafe impl<'a, T: ?Sized> Pointy for *const T { type Target = T; }

unsafe impl<'a, T: ?Sized> Pointy for *mut T { type Target = T; }

unsafe impl<'a, T: ?Sized> Pointy for Arc<T> { type Target = T; }

unsafe impl<'a, T: ?Sized> Pointy for Rc<T> { type Target = T; }

unsafe impl<'a, T: ?Sized> Pointy for sync::Weak<T> { type Target = T; }

unsafe impl<'a, T: ?Sized> Pointy for rc::Weak<T> { type Target = T; }

unsafe impl<'a, T: ?Sized> Pointy for Box<T> { type Target = T; }

#[test]
fn test_pointy() {
    #[derive(Eq, Ord, PartialEq, PartialOrd, Debug)]
    struct A;
    #[derive(Eq, Ord, PartialEq, PartialOrd, Debug)]
    struct B;
    trait X: RawAny {}

    impl X for A {}
    impl X for B {}
    let a: Box<dyn X> = box A;
    let b: Box<dyn X> = box B;
    assert_eq!(box A, downcast_sized::<_, Box<A>>(a).unwrap());
    let err = downcast_sized::<_, Box<A>>(b).unwrap_err();
    assert_eq!(err.from, "util::any::test_pointy::B");
    assert_eq!(err.to, "util::any::test_pointy::A");
}

#[test]
fn test_pointy2() {
    trait X: RawAny + 'static {
        fn x_type_info(self: *const F<Self>) -> TypeInfo { TypeInfo::of::<F<Self>>() }
    }
    #[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct A;
    #[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct B;
    #[derive(Eq, Ord, PartialEq, PartialOrd, Debug)]
    struct F<T: ?Sized>(usize, T);
    impl<T: ?Sized> Deref for F<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target { &self.1 }
    }
    impl X for A {}
    impl X for B {}
    unsafe impl RawAny for F<dyn X> {
        fn raw_type_info(self: *const Self) -> TypeInfo {
            self.x_type_info()
        }
    }

    let a: F<A> = F(1, A);
    let x: &F<dyn X> = &a;

    assert_eq!(&a, downcast_sized::<&F<dyn X>, &F<A>>(x).unwrap());
    let err = downcast_sized::<&F<dyn X>, &F<B>>(x).unwrap_err();
    assert_eq!(err.from, "util::any::test_pointy2::F<util::any::test_pointy2::A>");
    assert_eq!(err.to, "util::any::test_pointy2::F<util::any::test_pointy2::B>");
}

#[test]
fn test_pointy_ref() {
    trait Extension: RawAny {
        fn foo(&self) {}
    }
    impl Extension for i32 {}
    impl Extension for u32 {}
    let mut x = 1u32;
    {
        let y: &dyn Extension = &x;
        assert_eq!(&1u32, downcast_sized::<_, &u32>(y).unwrap());
        let err = downcast_sized::<_, &i32>(y).unwrap_err();
        assert_eq!(err.from, "u32");
        assert_eq!(err.from, "i32");
    }
    {
        let z: &mut dyn Extension = &mut x;
        assert_eq!(&mut 1u32, downcast_sized::<_, &mut u32>(&mut *z).unwrap());
        let err = downcast_sized::<_, &mut i32>(z).unwrap_err();
        assert_eq!(err.from, "u32");
        assert_eq!(err.from, "i32");
    }
}

