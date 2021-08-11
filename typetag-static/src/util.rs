use std::marker::PhantomData;

pub struct AnySingleton<T>(PhantomData<fn() -> T>);

impl<T> AnySingleton<T> {
    pub const fn new() -> Self { AnySingleton(PhantomData) }
}