#![feature(unboxed_closures)]
#![feature(type_alias_impl_trait)]
#![feature(fn_traits)]
#![feature(associated_type_bounds)]
#![feature(generic_associated_types)]
#![feature(unsized_fn_params)]

use std::future::Future;
#[macro_export]
macro_rules! define_async_fn_once {
    (
        #[macro_name($macro:ident)]
        #[path($($path:tt)*)]
        pub trait $trait:ident {
            async fn $method_name:ident<$lt:lifetime>(self, $($input:ident: $input_ty:ty),*);
        }
    ) => {
        pub trait $trait {
            type Output;
            type Fut<$lt> : ::std::future::Future<Output = Self::Output>;
            fn $method_name<$lt>(self, $($input: $input_ty),*) -> Self::Fut<$lt>;
        }
        #[macro_export]
        macro_rules! $macro {
            ({
                $$(
                    let $capture:ident: $capture_ty:ty = $capture_expr:expr;
                )*
                |$($$ $input:ident),*| -> $output:ty {$$($body:tt)*}
            }) => {{
                struct Foo<'c> {
                    $$(
                        $capture: $capture_ty,
                    )*
                    phantom: ::std::marker::PhantomData<&'c ()>,
                }
                impl<'c> $($path)* for Foo<'c> {
                    type Output = $output;
                    type Fut<$lt> = impl  ::std::future::Future<Output=Self::Output>;
                    fn $method_name<$lt>(self, $($$ $input: $input_ty),*) -> Self::Fut<$lt>
                    {
                        async move {
                            let Foo {
                                $$(
                                    $capture,
                                )*
                                phantom:_
                            } = self;
                            $$($body)*
                        }
                    }
                }
                Foo {
                    $$(
                        $capture: $capture_expr,
                    )*
                    phantom: ::std::marker::PhantomData
                }
            }};
        }
    };
}

#[cfg(test)]
pub mod test {
    define_async_fn_once! {
        #[macro_name(my_async_fn)]
        #[path($crate::test::MyAsyncFn)]
        pub trait MyAsyncFn{
            async fn my_call_async<'a>(self, input: &'a u8, input2: &'a u16);
        }
    }
    #[tokio::test]
    async fn test() {
        async fn bar<F: MyAsyncFn>(f: F) {
            println!("A");
            f.my_call_async(&123u8, &124u16).await;
        }
        let foo = String::new();
        bar(my_async_fn!({
            let foo: &'c String = &foo;
            let foo2: &'c String = &foo;
            |a, b| -> () {
                println!("hi {:?} {:?} {:?} {:?}", foo, a, b, foo2);
            }
        }))
        .await;
    }
}

//
// use std::future::Future;
//
// pub fn add(left: usize, right: usize) -> usize { left + right }
//
pub trait AsyncFn<'call, Args> {
    type Output;
    type Fut: Future<Output = Self::Output>;
    fn call_async(&'call self, args: Args) -> Self::Fut;
}

impl<'call, Args, F> AsyncFn<'call, Args> for F
where
    F: Fn<Args>,
    F::Output: Future,
{
    type Output = <<F as FnOnce<Args>>::Output as Future>::Output;
    type Fut = <F as FnOnce<Args>>::Output;

    fn call_async(&'call self, args: Args) -> Self::Fut { self.call(args) }
}

pub trait AsyncFnOnce<Args> {
    type Output;
    type Fut: Future<Output = Self::Output>;
    fn call_once_async(self, args: Args) -> Self::Fut;
}

impl<Args, F> AsyncFnOnce<Args> for F
where
    F: FnOnce<Args>,
    F::Output: Future,
{
    type Output = <<F as FnOnce<Args>>::Output as Future>::Output;
    type Fut = <F as FnOnce<Args>>::Output;
    fn call_once_async(self, args: Args) -> Self::Fut { self.call_once(args) }
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

pub trait AsyncFnOnce0: AsyncFnOnce<()> {
    fn call_once_async0(self) -> Self::Fut { self.call_once_async(()) }
}

impl<F: AsyncFnOnce<()>> AsyncFnOnce0 for F {}

pub trait AsyncFnOnce1<T>: AsyncFnOnce<(T,)> {
    fn call_once_async1(self, arg0: T) -> Self::Fut { self.call_once_async((arg0,)) }
}

impl<F: AsyncFnOnce<(T,)>, T> AsyncFnOnce1<T> for F {}

pub trait AsyncFnOnce2<T1, T2>: AsyncFnOnce<(T1, T2)> {
    fn call_once_async2(self, arg0: T1, arg1: T2) -> Self::Fut {
        self.call_once_async((arg0, arg1))
    }
}

impl<'a, F: AsyncFnOnce<(T1, T2)>, T1, T2> AsyncFnOnce2<T1, T2> for F {}

//
// #[macro_export]
// macro_rules! async_fn_impl {
//     (
//         #[trait($($tr:tt)*)]
//         #[method($method:ident)]
//         #[self($($self:tt)*)]
//         #[self_value($($self_value:tt)*)]
//         #[
//             capture($(
//                 $capture:ident: $capture_ty:ty = $capture_expr:expr
//             ),*)
//         ]
//         async fn $name:ident<$lt:lifetime> (
//             $(
//                 $arg:ident: $arg_ty:ty
//             ),*
//         ) -> $output:ty
//             {
//                 $($body:tt)*
//             }
//     ) => {{
//         #[allow(non_camel_case_types)]
//         struct $name<'capture>{
//             $(
//                 $capture:$capture_ty,
//             )*
//             phantom: ::std::marker::PhantomData<&'capture ()>,
//         }
//         impl<$lt,'capture:$lt> $($tr)*<'a,($($arg_ty,)*)> for $name<'capture> {
//             type Output = $output;
//             type Fut = impl 'a +Send+::std::future::Future<Output=Self::Output>;
//             fn $method($($self)*, ($($arg,)*):($($arg_ty,)*)) -> Self::Fut {
//                 async move {
//                     let $name{
//                         $(
//                             $capture,
//                         )*
//                         phantom: _
//                     } = $($self_value)*;
//                     {$($body)*}
//                 }
//             }
//         }
//         $name{
//             $(
//                 $capture : $capture_expr,
//             )*
//             phantom: ::std::marker::PhantomData
//         }
//     }};
// }
//
// #[macro_export]
// macro_rules! async_fn {
//     ($($x:tt)*) => {
//         async_fn_impl!(
//             #[trait($crate::AsyncFn)]
//             #[method(call_async)]
//             #[self(&'a self)]
//             #[self_value(self)]
//             $($x)*
//         )
//     };
// }
//
// #[macro_export]
// macro_rules! async_fn_once {
//     ($($x:tt)*) => {
//         async_fn_impl!(
//             #[trait($crate::AsyncFnOnce)]
//             #[method(call_once_async)]
//             #[self(self)]
//             #[self_value(self)]
//             $($x)*
//         )
//     };
// }
//
// #[tokio::test]
// async fn test() {
//     struct Foo<'cap> {
//         x: &'cap String,
//     };
//     impl<'cap: 'call, 'call> AsyncFn<'cap, 'call, (&'call usize,)> for Foo<'cap> {
//         type Output = &'call usize;
//         type Fut = impl 'call + Send + Future<Output = &'call usize>;
//         fn call_async(&'call self, (x,): (&'call usize,)) -> Self::Fut {
//             async move {
//                 println!("{:?}", x);
//                 x
//             }
//         }
//     }
//     async fn bar<
//         'cap,
//         F: for<'call> AsyncFn<'cap, 'call, (&'call usize,), Output = &'call usize>,
//     >(
//         f: F,
//     ) {
//         println!("{:?}", f.call_async1(&1usize).await)
//     }
//     let x = String::new();
//     fn foo<'cap>(x: &'cap String) { bar::<'cap, Foo>(Foo { x: x }); }
//     // let foo = Foo;
//     // foo.call_async0().await;
//     // async_fn!(
//     //     #[capture()]
//     //     async fn foo<'a>(x: &'a u8) -> () {
//     //         println!("{:?}", x);
//     //     }
//     // );
//     // async_fn!(
//     //     #[capture()]
//     //     async fn foo<'a>(x: &'a u8) -> &'a u8 { std::future::ready(x).await }
//     // );
//     // let captive = String::new();
//     // let x = async_fn!(
//     //     #[capture(cap: &'capture String = &captive, a: usize = 2)]
//     //     async fn foo<'a>(x: &'a u8) -> &'a u8 {
//     //         println!("hi {} {}", cap, a);
//     //         std::future::ready(x).await
//     //     }
//     // );
//     // async fn bar<F: for<'a> AsyncFn<'a, (&'a u8,), Output = &'a u8>>(f: F) {}
//     // bar(x);
//     // println!("{:?}", x.call_async1(&0).await);
// }
//
// // #[tokio::test]
// // async fn test2() {
// //     async fn bar<F>(f: F)
// //     where
// //         F: for<'a> AsyncFn1<'a, &'a usize, Output = &'a usize>,
// //     {
// //         f.call_async((&2,)).await;
// //     }
// //     let foo = String::new();
// //     let baz = String::new();
// //     // bar(async_fn_once!(
// //     //     #[capture(foo:String = foo, baz:&'capture String = &baz)]
// //     //     async fn foo<'a>(x: &'a usize) -> &'a usize {
// //     //         let foo: String = foo;
// //     //         let baz: &String = baz;
// //     //         println!("{:?} {:?} {:?}", x, foo, baz);
// //     //         x
// //     //     }
// //     // ))
// //     // .await;
// //     bar({
// //         #[allow(non_camel_case_types)]
// //         struct foo<'capture> {
// //             foo: String,
// //             baz: &'capture String,
// //             phantom: ::std::marker::PhantomData<&'capture ()>,
// //         }
// //         impl<'capture: 'a, 'a> crate::AsyncFn<'a, (&'a usize,)> for foo<'capture> {
// //             type Output = &'a usize;
// //             type Fut = impl 'a + Send + ::std::future::Future<Output = Self::Output>;
// //
// //             fn call_async(&'a self, (x,): (&'a usize,)) -> Self::Fut {
// //                 async move {
// //                     let foo {
// //                         foo,
// //                         baz,
// //                         phantom: _,
// //                     } = self;
// //                     {
// //                         let foo: &String = foo;
// //                         let baz: &&String = baz;
// //                         {
// //                             println!("{:?} {:?} {:?}", x, foo, baz);
// //                         };
// //                         x
// //                     }
// //                 }
// //             }
// //         }
// //         foo {
// //             foo: foo,
// //             baz: &baz,
// //             phantom: ::std::marker::PhantomData,
// //         }
// //     })
// //     .await;
// // }
