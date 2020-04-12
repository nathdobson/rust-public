use std::marker::Unsize;

pub trait Union2<A: ?Sized + 'static, B: ?Sized + 'static> {
    fn upcast1(&self) -> &A;
    fn upcast1_mut(&mut self) -> &mut A;
    fn upcast2(&self) -> &B;
    fn upcast2_mut(&mut self) -> &mut B;
}

impl<T, A: ?Sized + 'static, B: ?Sized + 'static> Union2<A, B> for T where T: Unsize<A> + Unsize<B> {
    fn upcast1(&self) -> &A {
        self
    }
    fn upcast1_mut(&mut self) -> &mut A {
        self
    }
    fn upcast2(&self) -> &B {
        self
    }
    fn upcast2_mut(&mut self) -> &mut B {
        self
    }
}
