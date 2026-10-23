//! `VecMap` / `VecSet`: ordered maps on a sorted `Vec`, for the engines' small per-window
//! and per-timer tables.
//!
//! They keep `BTreeMap` / `BTreeSet` semantics (keys unique, iteration in key order) but
//! each instantiation is a binary search plus `Vec::insert` / `remove` instead of the
//! B-tree node code, which is several KB per key/value type on wasm. The tables hold a
//! handful of entries, so the O(n) insert does not matter.

use core::fmt;

/// Sorted-`Vec` map with the `BTreeMap` subset the engines use.
#[derive(Clone, PartialEq)]
pub struct VecMap<K, V>(Vec<(K, V)>);

impl<K, V> Default for VecMap<K, V> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for VecMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.0.iter().map(|(k, v)| (k, v))).finish()
    }
}

type Pair<'a, K, V> = fn(&'a (K, V)) -> (&'a K, &'a V);
type PairMut<'a, K, V> = fn(&'a mut (K, V)) -> (&'a K, &'a mut V);

impl<K, V> VecMap<K, V> {
    /// As `BTreeMap`/`BTreeSet` `new`.
    pub const fn new() -> Self {
        Self(Vec::new())
    }
    /// As `BTreeMap`/`BTreeSet` `len`.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// As `BTreeMap`/`BTreeSet` `is_empty`.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// As `BTreeMap`/`BTreeSet` `clear`.
    pub fn clear(&mut self) {
        self.0.clear()
    }
    /// As `BTreeMap`/`BTreeSet` `iter`.
    pub fn iter(&self) -> core::iter::Map<core::slice::Iter<'_, (K, V)>, Pair<'_, K, V>> {
        self.0.iter().map(|(k, v)| (k, v))
    }
    /// As `BTreeMap`/`BTreeSet` `iter_mut`.
    pub fn iter_mut(&mut self) -> core::iter::Map<core::slice::IterMut<'_, (K, V)>, PairMut<'_, K, V>> {
        self.0.iter_mut().map(|(k, v)| (&*k, v))
    }
    /// As `BTreeMap`/`BTreeSet` `keys`.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &K> + ExactSizeIterator {
        self.0.iter().map(|(k, _)| k)
    }
    /// As `BTreeMap`/`BTreeSet` `values`.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &V> + ExactSizeIterator {
        self.0.iter().map(|(_, v)| v)
    }
    /// As `BTreeMap`/`BTreeSet` `values_mut`.
    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut V> + ExactSizeIterator {
        self.0.iter_mut().map(|(_, v)| v)
    }
    /// As `BTreeMap`/`BTreeSet` `first_key_value`.
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        self.0.first().map(|(k, v)| (k, v))
    }
    /// As `BTreeMap`/`BTreeSet` `pop_first`.
    pub fn pop_first(&mut self) -> Option<(K, V)> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }
    /// As `BTreeMap`/`BTreeSet` `retain`.
    pub fn retain(&mut self, mut f: impl FnMut(&K, &mut V) -> bool) {
        self.0.retain_mut(|(k, v)| f(k, v))
    }
}

impl<K: Ord, V> VecMap<K, V> {
    fn find(&self, k: &K) -> Result<usize, usize> {
        self.0.binary_search_by(|(x, _)| x.cmp(k))
    }
    /// As `BTreeMap`/`BTreeSet` `get`.
    pub fn get(&self, k: &K) -> Option<&V> {
        self.find(k).ok().map(|i| &self.0[i].1)
    }
    /// As `BTreeMap`/`BTreeSet` `get_mut`.
    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.find(k).ok().map(|i| &mut self.0[i].1)
    }
    /// As `BTreeMap`/`BTreeSet` `contains_key`.
    pub fn contains_key(&self, k: &K) -> bool {
        self.find(k).is_ok()
    }
    /// Like `BTreeMap::insert`: returns the old value; the stored key is kept.
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        match self.find(&k) {
            Ok(i) => Some(core::mem::replace(&mut self.0[i].1, v)),
            Err(i) => {
                self.0.insert(i, (k, v));
                None
            }
        }
    }
    /// As `BTreeMap`/`BTreeSet` `remove`.
    pub fn remove(&mut self, k: &K) -> Option<V> {
        self.find(k).ok().map(|i| self.0.remove(i).1)
    }
    /// As `BTreeMap`/`BTreeSet` `entry`.
    pub fn entry(&mut self, k: K) -> Entry<'_, K, V> {
        match self.find(&k) {
            Ok(i) => Entry::Occupied(OccupiedEntry { m: &mut self.0, i }),
            Err(i) => Entry::Vacant(VacantEntry { m: &mut self.0, i, k }),
        }
    }
}

/// As the `btree_map` `Entry`.
pub enum Entry<'a, K, V> {
    /// The key is there.
    Occupied(OccupiedEntry<'a, K, V>),
    /// The key is not there.
    Vacant(VacantEntry<'a, K, V>),
}

/// As the `btree_map` `OccupiedEntry`.
pub struct OccupiedEntry<'a, K, V> {
    m: &'a mut Vec<(K, V)>,
    i: usize,
}

/// As the `btree_map` `VacantEntry`.
pub struct VacantEntry<'a, K, V> {
    m: &'a mut Vec<(K, V)>,
    i: usize,
    k: K,
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    /// As `BTreeMap`/`BTreeSet` `get`.
    pub fn get(&self) -> &V {
        &self.m[self.i].1
    }
    /// As `BTreeMap`/`BTreeSet` `get_mut`.
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.m[self.i].1
    }
    /// As `BTreeMap`/`BTreeSet` `into_mut`.
    pub fn into_mut(self) -> &'a mut V {
        &mut self.m[self.i].1
    }
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    /// As `BTreeMap`/`BTreeSet` `insert`.
    pub fn insert(self, v: V) -> &'a mut V {
        self.m.insert(self.i, (self.k, v));
        &mut self.m[self.i].1
    }
}

impl<'a, K, V> Entry<'a, K, V> {
    /// As `BTreeMap`/`BTreeSet` `or_insert_with`.
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> &'a mut V {
        match self {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(f()),
        }
    }
    /// As `BTreeMap`/`BTreeSet` `or_default`.
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }
}

impl<'a, K, V> IntoIterator for &'a VecMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = core::iter::Map<core::slice::Iter<'a, (K, V)>, Pair<'a, K, V>>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut VecMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = core::iter::Map<core::slice::IterMut<'a, (K, V)>, PairMut<'a, K, V>>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<K, V> IntoIterator for VecMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Sorted-`Vec` set with the `BTreeSet` subset the engines use.
#[derive(Clone, PartialEq)]
pub struct VecSet<K>(Vec<K>);

impl<K> Default for VecSet<K> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<K: fmt::Debug> fmt::Debug for VecSet<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.0.iter()).finish()
    }
}

impl<K> VecSet<K> {
    /// As `BTreeMap`/`BTreeSet` `new`.
    pub const fn new() -> Self {
        Self(Vec::new())
    }
    /// As `BTreeMap`/`BTreeSet` `len`.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// As `BTreeMap`/`BTreeSet` `is_empty`.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// As `BTreeMap`/`BTreeSet` `iter`.
    pub fn iter(&self) -> core::slice::Iter<'_, K> {
        self.0.iter()
    }
}

impl<K: Ord> VecSet<K> {
    /// As `BTreeMap`/`BTreeSet` `contains`.
    pub fn contains(&self, k: &K) -> bool {
        self.0.binary_search(k).is_ok()
    }
    /// Like `BTreeSet::insert`: `true` when the key was not there.
    pub fn insert(&mut self, k: K) -> bool {
        match self.0.binary_search(&k) {
            Ok(_) => false,
            Err(i) => {
                self.0.insert(i, k);
                true
            }
        }
    }
    /// As `BTreeMap`/`BTreeSet` `remove`.
    pub fn remove(&mut self, k: &K) -> bool {
        match self.0.binary_search(k) {
            Ok(i) => {
                self.0.remove(i);
                true
            }
            Err(_) => false,
        }
    }
}

impl<K> IntoIterator for VecSet<K> {
    type Item = K;
    type IntoIter = std::vec::IntoIter<K>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K> IntoIterator for &'a VecSet<K> {
    type Item = &'a K;
    type IntoIter = core::slice::Iter<'a, K>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn behaves_like_btree() {
        let mut s = 11u64;
        let mut a = VecMap::new();
        let mut b = BTreeMap::new();
        let mut sa = VecSet::new();
        let mut sb = BTreeSet::new();
        for i in 0..2000u64 {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let k = (s >> 40) % 37;
            match (s >> 20) % 5 {
                0 | 1 => assert_eq!(a.insert(k, i), b.insert(k, i)),
                2 => assert_eq!(a.remove(&k), b.remove(&k)),
                3 => *a.entry(k).or_default() += 1,
                _ => assert_eq!(a.pop_first(), b.pop_first()),
            }
            if (s >> 20) % 5 == 3 {
                *b.entry(k).or_default() += 1;
            }
            assert_eq!(sa.insert(k % 7), sb.insert(k % 7));
            if i % 3 == 0 {
                assert_eq!(sa.remove(&(k % 5)), sb.remove(&(k % 5)));
            }
            assert_eq!(a.iter().collect::<Vec<_>>(), b.iter().collect::<Vec<_>>());
            assert_eq!(a.get(&k), b.get(&k));
            assert_eq!(a.first_key_value(), b.first_key_value());
            assert_eq!(sa.iter().collect::<Vec<_>>(), sb.iter().collect::<Vec<_>>());
        }
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert_eq!(format!("{sa:?}"), format!("{sb:?}"));
        a.retain(|k, _| k % 2 == 0);
        b.retain(|k, _| k % 2 == 0);
        assert_eq!(a.into_iter().collect::<Vec<_>>(), b.into_iter().collect::<Vec<_>>());
    }
}
