#![allow(dead_code)]

use std::marker::PhantomData;

use proc_macro_helper_test::{my_attr_macro, MyClone, MyDerive};

#[test]
fn test_my_derive() {
    #[derive(MyDerive)]
    #[my_attr(my_key = 2)]
    struct Foo;
    assert_eq!(MY_DERIVE, 2);
}

#[test]
fn test_my_attr_macro() {
    #[my_attr_macro(my_key = 3)]
    struct Foo;
    assert_eq!(MY_ATTR_MACRO, 3);
}

#[test]
fn test_my_clone() {
    fn assert_clone<T: Clone>() {}
    struct NoClone;

    #[derive(MyClone)]
    struct Struct(u8, u16);
    assert_clone::<Struct>();

    #[derive(MyClone)]
    enum Enum {
        BarBar(u8),
        BarBarBar(u16),
    }
    assert_clone::<Enum>();

    #[derive(MyClone)]
    struct ParamDep<T> {
        value: T,
    }
    assert_clone::<ParamDep<()>>();
    // assert_clone::<ParamDep<NoClone>>();

    #[derive(MyClone)]
    struct ParamIndep<T> {
        value: PhantomData<T>,
    }
    assert_clone::<ParamIndep<()>>();
    assert_clone::<ParamIndep<NoClone>>();

    #[derive(MyClone)]
    enum Foo<T> {
        A,
        B(u8, T),
        C { u32: u16, u64: T },
    }
}
