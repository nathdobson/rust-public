use std::collections::BTreeMap;
use std::ops::{Range, RangeBounds, Bound};
use std::borrow::Borrow;
use std::fmt;
use std::fmt::Debug;

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
struct Entry<K, V> {
    end: K,
    value: V,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct RangeMap<K: Ord, V> {
    inner: BTreeMap<K, Entry<K, V>>,
}

impl<K: Ord, V> RangeMap<K, V> {
    pub fn new() -> Self {
        RangeMap {
            inner: BTreeMap::new()
        }
    }
    pub fn erase<'a, 'b, Q: 'b, R: RangeBounds<&'b Q> + Clone>(&'a mut self, range: R) where K: Clone + Borrow<Q>, Q: Ord {
        loop {
            let remove: K =
                if let Some((r, _)) = self.range(range.clone()).next() {
                    r.start.clone()
                } else {
                    break;
                };
            self.inner.remove::<K>(&remove);
        }
    }
    pub fn erase_and_insert(&mut self, range: Range<K>, value: V) where K: Clone {
        self.erase(&range.start..&range.end);
        self.inner.insert(range.start, Entry { end: range.end, value });
    }
    pub fn iter(&self) -> impl Iterator<Item=(Range<&K>, &V)> {
        self.inner.iter().map(|(k, v)| { (k..&v.end, &v.value) })
    }
    pub fn values(&self) -> impl Iterator<Item=&V> {
        self.inner.values().map(|v| &v.value)
    }
    pub fn range<'a, 'b: 'a, Q, R>(&'a self, range: R)
                                   -> impl Iterator<Item=(Range<&'a K>, &'a V)> + 'a
        where K: Borrow<Q>,
              Q: Ord + 'b,
              R: RangeBounds<&'b Q> {
        let head = match range.start_bound() {
            Bound::Included(x) => {
                let before = ..*x;
                self.inner.range(before).next_back().filter(|(_, v)| {
                    x < &v.end.borrow()
                })
            }
            Bound::Excluded(_) => {
                unimplemented!()
            }
            Bound::Unbounded => None,
        };
        let tail = self.inner.range((range.start_bound().cloned(), range.end_bound().cloned()));
        head.into_iter().chain(tail).map(|(start, v)| {
            (start..&v.end, &v.value)
        })
    }
    pub fn get<'a, 'b: 'a, Q>(&'a self, key: &'b Q) -> Option<(Range<&'a K>, &'a V)>
        where K: Borrow<Q>,
              Q: Ord {
        self.range(key..=key).next()
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K: Ord + Debug, V: Debug> fmt::Debug for RangeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut map = f.debug_map();
        for x in self.iter() {
            map.entry(&x.0, x.1);
        }
        map.finish()?;
        Ok(())
    }
}

#[test]
fn test_range_map() {
    let mut map = RangeMap::new();
    map.erase_and_insert(10u8..20, "a");
    map.erase_and_insert(30u8..40, "b");
    map.erase_and_insert(50u8..60, "c");
    map.erase_and_insert(70u8..80, "d");
    map.erase_and_insert(90u8..100, "e");
    assert_eq!(map.len(), 5);
    {
        let mut map = map.clone();
        map.erase(&35u8..&75);
        assert_eq!(map.iter().collect::<Vec<_>>(),
                   vec![(&10u8..&20, &"a"), (&90u8..&100, &"e")]);
    }
    {
        let mut map = map.clone();
        map.erase(..&75);
        assert_eq!(map.iter().collect::<Vec<_>>(),
                   vec![(&90u8..&100, &"e")]);
    }
    {
        let mut map = map.clone();
        map.erase(&35u8..);
        assert_eq!(map.iter().collect::<Vec<_>>(),
                   vec![(&10u8..&20, &"a")]);
    }
    {
        let mut map = map.clone();
        map.erase(..);
        assert_eq!(map.iter().collect::<Vec<_>>(),
                   vec![]);
    }
}

#[test]
fn test_edge() {
    let mut map: RangeMap<u8, u8> = RangeMap::new();
    map.erase_and_insert(1..2, 10);
    map.erase_and_insert(2..3, 20);
    map.erase_and_insert(3..4, 30);
    map.erase_and_insert(2..3, 40);
    assert_eq!(map.iter().collect::<Vec<_>>(),
               vec![(&1..&2, &10), (&2..&3, &40), (&3..&4, &30)]);
}
