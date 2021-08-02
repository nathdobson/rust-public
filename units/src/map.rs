use std::iter::FromIterator;

#[derive(Eq, Ord, PartialEq, PartialOrd, Default, Hash, Debug, Clone)]
pub struct VecMap<K, V>(Vec<(K, V)>);

pub struct OccupiedEntry<'a, K: 'a, V: 'a> {
    map: &'a mut VecMap<K, V>,
    index: usize,
}

pub struct VacantEntry<'a, K: 'a, V: 'a> {
    map: &'a mut VecMap<K, V>,
    index: usize,
    key: K,
}

pub enum Entry<'a, K: 'a, V: 'a> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<K: Ord, V> VecMap<K, V> {
    pub fn new() -> Self { VecMap(vec![]) }
    pub fn iter(&self) -> impl Iterator<Item=(&K, &V)> {
        self.into_iter()
    }
    pub fn entry<'a>(&'a mut self, key: K) -> Entry<'a, K, V> {
        match self.0.binary_search_by_key(&&key, |(k, v)| k) {
            Ok(index) => Entry::Occupied(OccupiedEntry { map: self, index }),
            Err(index) => Entry::Vacant(VacantEntry { map: self, index, key: key })
        }
    }
    pub fn retain(&mut self, f: impl Fn(&K, &V) -> bool) {
        self.0.retain(|(k, v)| f(k, v));
    }
}

impl<'a, K: Ord + 'a, V: 'a> Entry<'a, K, V> {
    pub fn or_default(self) -> &'a mut V where V: Default {
        match self {
            Entry::Occupied(o) => o.get_mut(),
            Entry::Vacant(v) => v.insert(V::default())
        }
    }
}

impl<'a, K: Ord + 'a, V: 'a> OccupiedEntry<'a, K, V> {
    pub fn get_mut(self) -> &'a mut V {
        &mut self.map.0[self.index].1
    }
}

impl<'a, K: Ord + 'a, V: 'a> VacantEntry<'a, K, V> {
    pub fn insert(self, value: V) -> &'a mut V {
        self.map.0.insert(self.index, (self.key, value));
        &mut self.map.0[self.index].1
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for VecMap<K, V> {
    fn from_iter<T: IntoIterator<Item=(K, V)>>(iter: T) -> Self { VecMap(Vec::from_iter(iter)) }
}

impl<K: Ord, V> IntoIterator for VecMap<K, V> {
    type Item = (K, V);
    type IntoIter = <Vec<(K, V)> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}

impl<'a, K: Ord, V> IntoIterator for &'a VecMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = impl Iterator<Item=(&'a K, &'a V)>;
    fn into_iter(self) -> Self::IntoIter { self.0.iter().map(|x| (&x.0, &x.1)) }
}