use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub struct Tree<K: Hash + Eq> {
    parents: HashMap<K, K>,
    children: HashMap<K, HashSet<K>>,
    empty: HashSet<K>,
}

impl<K: Hash + Eq + Clone> Tree<K> {
    pub fn new() -> Self {
        Tree {
            parents: HashMap::new(),
            children: HashMap::new(),
            empty: HashSet::new(),
        }
    }
    pub fn set_parent(&mut self, parent: K, child: K) {
        if let Some(old_parent) = self.parents.insert(child.clone(), parent.clone()) {
            if let Some(old_children) = self.children.get_mut(&old_parent) {
                old_children.remove(&child);
            }
        }
        self.children.entry(parent).or_insert(HashSet::new()).insert(child);
    }
    pub fn remove(&mut self, child: K) {
        if let Some(old_parent) = self.parents.remove(&child) {
            if let Some(old_children) = self.children.get_mut(&old_parent) {
                old_children.remove(&child);
            }
        }
    }
    pub fn parent(&self, node: &K) -> Option<&K> {
        self.parents.get(node)
    }
    pub fn children<'a>(&'a self, node: &K) -> impl Iterator<Item=&'a K> + 'a {
        self.children.get(node).unwrap_or(&self.empty).iter()
    }
}