#![feature(unboxed_closures)]
#![feature(type_alias_impl_trait)]
#![feature(fn_traits)]

use std::future::Future;

pub fn add(left: usize, right: usize) -> usize { left + right }

pub trait AsyncFn<'a, Args>: Send {
    type Output: Send;
    type Fut: 'a + Send + Future<Output = Self::Output>;
    fn call_async(&'a self, args: Args) -> Self::Fut;
}

pub trait AsyncFn0<'a>: AsyncFn<'a, ()> {
    fn call_async0(&'a self) -> Self::Fut { self.call_async(()) }
}

impl<'a, F: AsyncFn<'a, ()>> AsyncFn0<'a> for F {}

pub trait AsyncFn1<'a, T>: AsyncFn<'a, (T,)> {
    fn call_async1(&'a self, arg0: T) -> Self::Fut { self.call_async((arg0,)) }
}

impl<'a, F: AsyncFn<'a, (T,)>, T> AsyncFn1<'a, T> for F {}

pub trait AsyncFn2<'a, T1, T2>: AsyncFn<'a, (T1, T2)> {
    fn call_async2(&'a self, arg0: T1, arg1: T2) -> Self::Fut { self.call_async((arg0, arg1)) }
}

impl<'a, F: AsyncFn<'a, (T1, T2)>, T1, T2> AsyncFn2<'a, T1, T2> for F {}

#[macro_export]
macro_rules! async_fn {
    (
        #[
            capture($(
                $capture:ident: $capture_ty:ty = $capture_expr:expr
            ),*)
        ]
        fn $name:ident<$lt:lifetime> (
            $(
                $arg:ident: $arg_ty:ty
            ),*
        ) -> $output:ty
            {
                $($body:tt)*
            }
    ) => {{
        #[allow(non_camel_case_types)]
        struct $name<'capture>{
            $(
                $capture:$capture_ty,
            )*
            phantom: ::std::marker::PhantomData<&'capture ()>,
        }
        impl<$lt,'capture:$lt> $crate::AsyncFn<'a,($($arg_ty,)*)> for $name<'capture>{
            type Output = $output;
            type Fut = impl 'a +Send+Future<Output=Self::Output>;
            fn call_async(&'a self, ($($arg,)*):($($arg_ty,)*)) -> Self::Fut {
                async move {
                    let $name{
                        $(
                            $capture,
                        )*
                        phantom: _
                    } = self;
                    {$($body)*}
                }
            }
        }
        $name{
            $(
                $capture : $capture_expr,
            )*
            phantom: ::std::marker::PhantomData
        }
    }};
}

#[tokio::test]
async fn test() {
    struct Foo;
    impl<'a> AsyncFn<'a, ()> for Foo {
        type Output = ();
        type Fut = impl 'a + Send + Future<Output = ()>;
        fn call_async(&'a self, _: ()) -> Self::Fut { async move { return () } }
    }
    let foo = Foo;
    foo.call_async0().await;
    async_fn!(
        #[capture()]
        fn foo<'a>(x: &'a u8) -> () {
            println!("{:?}", x);
        }
    );
    async_fn!(
        #[capture()]
        fn foo<'a>(x: &'a u8) -> &'a u8 { std::future::ready(x).await }
    );
    let captive = 1;
    let x = async_fn!(
        #[capture(cap: &'capture usize = &captive, a: usize = 2)]
        fn foo<'a>(x: &'a u8) -> &'a u8 {
            println!("hi {} {}", cap, a);
            std::future::ready(x).await
        }
    );
    println!("{:?}", x.call_async1(&0).await);
}
