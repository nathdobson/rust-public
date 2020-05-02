use std::cmp::Ordering;
use itertools::{Itertools, MergeJoinBy, EitherOrBoth};

pub trait Itertools2: Iterator {
    fn merge_keys<K, V1, V2, J>(self, other: J) -> MergeKeys<K, V1, V2, Self, J::IntoIter>
        where
            Self: Iterator<Item=(K, V1)>,
            J: IntoIterator<Item=(K, V2)>,
            Self: Sized,
            K: Ord,
    {
        MergeKeys {
            inner: self.merge_join_by(other, |left: &(K, V1), right: &(K, V2)| {
                left.0.cmp(&right.0)
            })
        }
    }
}

impl<T> Itertools2 for T where T: Iterator {}

pub struct MergeKeys<K, V1, V2, I, J> where I: Iterator<Item=(K, V1)>, J: Iterator<Item=(K, V2)> {
    inner: MergeJoinBy<I, J, fn(&(K, V1), &(K, V2)) -> Ordering>
}

impl<K, V1, V2, I, J> Iterator for MergeKeys<K, V1, V2, I, J> where
    I: Iterator<Item=(K, V1)>,
    J: Iterator<Item=(K, V2)> {
    type Item = (K, EitherOrBoth<V1, V2>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next()? {
            EitherOrBoth::Both((k1, v1), (_, v2)) => Some((k1, EitherOrBoth::Both(v1, v2))),
            EitherOrBoth::Left((k1, v1)) => Some((k1, EitherOrBoth::Left(v1))),
            EitherOrBoth::Right((k2, v2)) => Some((k2, EitherOrBoth::Right(v2))),
        }
    }
}

#[test]
fn test_merge_keys() {
    let foo = vec![(0, "a".to_string()), (1, "b".to_string())];
    let bar = vec![(0, "x".to_string()), (2, "y".to_string())];
    let result: Vec<(i32, String)> = foo.into_iter().merge_keys(bar.into_iter()).map(|(k, v)| {
        (k, v.reduce(|x, y| { format!("{}{}", x, y) }))
    }).collect();
    assert_eq!(result, vec![(0, "ax".to_string()), (1, "b".to_string()), (2, "y".to_string())]);
}