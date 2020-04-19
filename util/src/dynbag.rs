use std::any::Any;
use std::marker::{PhantomData, Unsize};

use crate::union::Union2;
use std::ops::{Index, IndexMut};
use crate::bag::{Bag, Token};

pub struct DynBag<T: ?Sized + 'static> {
    vec: Bag<dyn Union2<dyn Any, T>>,
}

pub struct DynToken<A: 'static + ?Sized> {
    index: Token,
    phantom: PhantomData<A>,
}

impl<T: ?Sized + 'static> DynBag<T> {
    pub fn new() -> Self {
        DynBag {
            vec: vec![],
        }
    }
    pub fn push<A>(&mut self, element: A) -> DynKey<A> where A: Unsize<T> + 'static {
        let result = DynKey { index: self.vec.len(), phantom: PhantomData };
        self.vec.push(Box::new(element));
        result
    }
    pub fn nth(&self, index: usize) -> &T {
        (&*self.vec[index]).upcast2()
    }
    pub fn nth_mut(&mut self, index: usize) -> &mut T {
        (&mut *self.vec[index]).upcast2_mut()
    }
}

impl<A: 'static, B: ?Sized> Index<DynKey<A>> for DynBag<B> {
    type Output = A;

    fn index(&self, key: DynKey<A>) -> &A {
        let result: &dyn Any = (&*self.vec[key.index]).upcast1();
        result.downcast_ref().unwrap()
    }
}

impl<A: 'static, B: ?Sized> IndexMut<DynKey<A>> for DynBag<B> {
    fn index_mut(&mut self, key: DynKey<A>) -> &mut A {
        let result: &mut dyn Any = (&mut *self.vec[key.index]).upcast1_mut();
        result.downcast_mut().unwrap()
    }
}

#[test]
fn test_dyn_vec() {
    trait Named {
        fn name(&self) -> &'static str;
    }
    #[derive(Eq, Ord, PartialOrd, PartialEq)]
    struct A;

    impl Named for A {
        fn name(&self) -> &'static str {
            "A"
        }
    }
    #[derive(Eq, Ord, PartialOrd, PartialEq)]
    struct B;
    impl Named for B {
        fn name(&self) -> &'static str {
            "B"
        }
    }

    let mut vec: DynBag<dyn Named> = DynBag::new();
    let a = vec.push(A);
    let b = vec.push(B);
    assert!(vec[a] == A);
    assert!(vec[b] == B);
    assert_eq!("A", vec.nth(0).name());
    assert_eq!("B", vec.nth(1).name());
}