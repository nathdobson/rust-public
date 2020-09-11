use std::sync::atomic::{Ordering, AtomicU128, AtomicU64, AtomicUsize};
use std::marker::PhantomData;
use std::mem;
use std::mem::size_of;
use std::sync::Mutex;

pub struct Atomic<T, R: RawAtomic, E: Encoder<T, <R as RawAtomic>::Value> = Transmuter<T, <R as RawAtomic>::Value>>(R, PhantomData<(T, E)>);

pub struct Transmuter<T, R>(PhantomData<(T, R)>);

pub trait Encoder<T, V> {
    unsafe fn encode(x: T) -> V;
    unsafe fn decode(x: V) -> T;
}

pub unsafe trait RawAtomic {
    type Value;
    fn new(value: Self::Value) -> Self;
    fn load(&self, ordering: Ordering) -> Self::Value;
    fn store(&self, new: Self::Value, ordering: Ordering);
    fn compare_exchange_weak(&self,
                             current: Self::Value,
                             new: Self::Value,
                             success: Ordering,
                             failure: Ordering) -> Result<Self::Value, Self::Value>;
    fn swap(&self, new: Self::Value, ordering: Ordering) -> Self::Value;
    fn fetch_update(&self, set_order: Ordering, fetch_order: Ordering, f: impl FnMut(Self::Value)
        -> Option<Self::Value>) -> Result<Self::Value, Self::Value>;
}

unsafe impl RawAtomic for AtomicU128 {
    type Value = u128;
    fn new(value: Self::Value) -> Self {
        AtomicU128::new(value)
    }

    fn load(&self, ordering: Ordering) -> Self::Value {
        self.load(ordering)
    }

    fn store(&self, new: Self::Value, ordering: Ordering) {
        self.store(new, ordering)
    }

    fn compare_exchange_weak(&self,
                             current: Self::Value,
                             new: Self::Value,
                             success: Ordering,
                             failure: Ordering) -> Result<Self::Value, Self::Value> {
        self.compare_exchange_weak(current, new, success, failure)
    }
    fn swap(&self, new: Self::Value, ordering: Ordering) -> Self::Value {
        self.swap(new, ordering)
    }
    fn fetch_update(&self, set_order: Ordering, fetch_order: Ordering, f: impl FnMut(Self::Value)
        -> Option<Self::Value>) -> Result<Self::Value, Self::Value> {
        self.fetch_update(set_order, fetch_order, f)
    }
}

unsafe impl RawAtomic for AtomicU64 {
    type Value = u64;
    fn new(value: Self::Value) -> Self {
        AtomicU64::new(value)
    }

    fn load(&self, ordering: Ordering) -> Self::Value {
        self.load(ordering)
    }

    fn store(&self, new: Self::Value, ordering: Ordering) {
        self.store(new, ordering)
    }

    fn compare_exchange_weak(&self,
                             current: Self::Value,
                             new: Self::Value,
                             success: Ordering,
                             failure: Ordering) -> Result<Self::Value, Self::Value> {
        self.compare_exchange_weak(current, new, success, failure)
    }
    fn swap(&self, new: Self::Value, ordering: Ordering) -> Self::Value {
        self.swap(new, ordering)
    }
    fn fetch_update(&self, set_order: Ordering, fetch_order: Ordering, f: impl FnMut(Self::Value)
        -> Option<Self::Value>) -> Result<Self::Value, Self::Value> {
        self.fetch_update(set_order, fetch_order, f)
    }
}

unsafe impl RawAtomic for AtomicUsize {
    type Value = usize;
    fn new(value: Self::Value) -> Self {
        AtomicUsize::new(value)
    }

    fn load(&self, ordering: Ordering) -> Self::Value {
        self.load(ordering)
    }

    fn store(&self, new: Self::Value, ordering: Ordering) {
        self.store(new, ordering)
    }

    fn compare_exchange_weak(&self,
                             current: Self::Value,
                             new: Self::Value,
                             success: Ordering,
                             failure: Ordering) -> Result<Self::Value, Self::Value> {
        self.compare_exchange_weak(current, new, success, failure)
    }
    fn swap(&self, new: Self::Value, ordering: Ordering) -> Self::Value {
        self.swap(new, ordering)
    }
    fn fetch_update(&self, set_order: Ordering, fetch_order: Ordering, f: impl FnMut(Self::Value)
        -> Option<Self::Value>) -> Result<Self::Value, Self::Value> {
        self.fetch_update(set_order, fetch_order, f)
    }
}

unsafe fn force_transmute<T, U>(value: T) -> U {
    assert_eq!(size_of::<T>(), size_of::<U>());
    let result = mem::transmute_copy(&value);
    mem::forget(value);
    result
}

impl<T, V> Encoder<T, V> for Transmuter<T, V> {
    unsafe fn encode(x: T) -> V {
        force_transmute(x)
    }

    unsafe fn decode(x: V) -> T {
        force_transmute(x)
    }
}

impl<T, R: RawAtomic, E: Encoder<T, R::Value>> Atomic<T, R, E> {
    pub fn new(x: T) -> Self {
        unsafe {
            Atomic(R::new(E::encode(x)), PhantomData)
        }
    }
    pub fn load(&self, ordering: Ordering) -> T where T: Copy {
        unsafe { E::decode(self.0.load(ordering)) }
    }
    pub fn store(&self, new: T, ordering: Ordering) where T: Copy {
        unsafe { self.0.store(E::encode(new), ordering) }
    }
    pub fn compare_exchange_weak(&self,
                                 current: T,
                                 new: T,
                                 success: Ordering,
                                 failure: Ordering) -> Result<T, T> {
        unsafe {
            match self.0.compare_exchange_weak(
                E::encode(current),
                E::encode(new),
                success,
                failure) {
                Ok(ok) => Ok(E::decode(ok)),
                Err(err) => Err(E::decode(err)),
            }
        }
    }
    pub fn swap(&self, new: T, ordering: Ordering) -> T {
        unsafe {
            E::decode(self.0.swap(E::encode(new), ordering))
        }
    }
    pub fn fetch_update(&self, set_order: Ordering, fetch_order: Ordering, mut f: impl FnMut(T) -> Option<T>) -> Result<T, T> {
        unsafe {
            match self.0.fetch_update(
                set_order, fetch_order,
                |x| {
                    f(E::decode(x)).map(|x| E::encode(x))
                }) {
                Ok(ok) => Ok(E::decode(ok)),
                Err(err) => (Err(E::decode(err))),
            }
        }
    }
}
