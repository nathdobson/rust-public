#![feature(unboxed_closures)]
#![feature(const_fn)]
#![feature(log_syntax)]
#![feature(trace_macros)]

use std::future::Future;
use std::marker::PhantomData;
use std::mem::size_of;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct TraitFuture<'a, R: Future> {
    raw: R,
    phantom: PhantomData<&'a ()>,
}

impl<'a, R: Future> TraitFuture<'a, R> {
    pub fn new(raw: R) -> Self {
        TraitFuture {
            raw,
            phantom: PhantomData,
        }
    }
}

impl<'a, R: Future> Future for TraitFuture<'a, R> {
    type Output = R::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().raw).poll(cx) }
    }
}

pub unsafe fn poll_erased<F, A>(
    _: F,
    this: *mut (),
    cx: &mut Context,
) -> Poll<<F::Output as Future>::Output>
where
    F: Fn<A>,
    F::Output: Future,
{
    F::Output::poll(Pin::new_unchecked(&mut *(this as *mut F::Output)), cx)
}

pub const fn future_size<T: Fn<A>, A>(_: &T) -> usize { size_of::<T::Output>() }

#[macro_export]
macro_rules! async_trait {
    {
        $vis:vis trait $trait:ident {
            $(
                async <$($blt0:lifetime $(+ $blt:lifetime)*)? >
                fn $method:ident $(<$($lt:lifetime),*>)? ($($arg:tt)*)
                $(-> $ret:ty)?;
            )*
        }
    } => {
        paste::paste!{
            $vis trait $trait {
                $(
                    #[allow(non_camel_case_types)]
                    type [<Future_ $method>] : std::future::Future + Send;
                    fn $method $(<$($lt),*>)? ($($arg)*) $(-> $ret)?
                    -> $crate::TraitFuture<$($blt0 $(+ $blt)*)?,Self::[<Future_ $method>]>;
                )*
            }
        }
    }
}

#[macro_export]
macro_rules! async_trait_impl {
    {
        impl $trait:ident for $impl:ty {
            $(
                async <$($blt0:lifetime $(+ $blt:lifetime)*)? >
                fn $method:ident $(<$($lt:lifetime),*>)? ($($arg:ident : $argt:ty),*)
                -> $ret:ty
                $body:block
            )*
        }
    } => {
        paste::paste!{
            impl $impl {
                $(
                    fn [<$method _impl>]
                    $(<$($lt),*>)? ($($arg : $argt),*)
                    -> impl std::future::Future<Output=$ret> + Send + $($blt0 $(+ $blt)*)?
                    { async move { $body } }
                )*
            }

            $(
                #[allow(non_camel_case_types)]
                struct [<Future_ $method _impl>] (
                    std::mem::MaybeUninit<[u8; { $crate::future_size(&$impl::[<$method _impl>]) }]>
                );
                impl std::future::Future for [<Future_ $method _impl>] {
                    type Output = $ret;
                    fn poll(self: std::pin::Pin<&mut Self>, cx:&mut std::task::Context) -> std::task::Poll<Self::Output>{
                        unsafe {
                            $crate::poll_erased(&$impl::[<$method _impl>],
                                        self.get_unchecked_mut().0.as_mut_ptr() as *mut (),
                                        cx)
                        }
                    }
                }
            )*

            impl $trait for $impl {
                $(
                    type [<Future_ $method>] = [<Future_ $method _impl>];
                    fn $method $(<$($lt),*>)? ($($arg : $argt),*)
                    -> $crate::TraitFuture<$($blt0 $(+ $blt)*)?,Self::[<Future_ $method>]> {
                        unsafe{
                            $crate::TraitFuture::new([<Future_ $method _impl>](std::mem::transmute(Self::[<$method _impl>]($($arg),*))))
                        }
                    }
                )*
            }
        }
    }
}

#[cfg(test)]
mod tests {
    async_trait! {
        trait Dragon {
            async<'static> fn burninate1(self: &Self, x: u16, y: u8);
        }
    }

    #[derive(Debug)]
    struct Trogdor;

    async_trait_impl!(
        impl Dragon for Trogdor {
            async<'_> fn burninate1(self: &Self, x: u16, y: u8) -> () {
                println!("{:?} {} {}", self, x, y);
            }
        }
    );

    #[test]
    fn test_trogdor() {
        use futures::executor::block_on;
        block_on(Trogdor.burninate(1, 2));
    }
}
