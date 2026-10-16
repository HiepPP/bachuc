//! `IdMap` and `IdSet`: a sorted `Vec` with binary search, keyed by TASK IDs. A plan holds tens of
//! TASKs, and in a debug build a `HashMap` of 40 IDs cost six times the lookups of this; the
//! timeline and the plan tree build several per redraw of the derived data. An insert of a key
//! that is there replaces its value, as JS `Map.set` does.

use std::borrow::Borrow;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct IdMap<K, V> {
    entries: Vec<(K, V)>,
}

impl<K, V> Default for IdMap<K, V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<K: Ord, V> IdMap<K, V> {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    fn find<Q: Ord + ?Sized>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
    {
        self.entries
            .binary_search_by(|(probe, _)| probe.borrow().cmp(key))
    }

    pub(crate) fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.find(&key) {
            Ok(index) => Some(std::mem::replace(&mut self.entries[index].1, value)),
            Err(index) => {
                self.entries.insert(index, (key, value));
                None
            }
        }
    }

    pub(crate) fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.find(key).ok().map(|index| &self.entries[index].1)
    }

    pub(crate) fn contains_key<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.find(key).is_ok()
    }

    pub(crate) fn remove<Q: Ord + ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        self.find(key)
            .ok()
            .map(|index| self.entries.remove(index).1)
    }

    /// The value of `key`, inserting the default first when it is missing.
    pub(crate) fn get_or_default(&mut self, key: K) -> &mut V
    where
        V: Default,
    {
        let index = match self.find(&key) {
            Ok(index) => index,
            Err(index) => {
                self.entries.insert(index, (key, V::default()));
                index
            }
        };
        &mut self.entries[index].1
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, value)| value)
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for IdMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(items: I) -> Self {
        let items = items.into_iter();
        let mut map = Self::with_capacity(items.size_hint().0);
        for (key, value) in items {
            map.insert(key, value);
        }
        map
    }
}

impl<K, V> IntoIterator for IdMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct IdSet<K> {
    map: IdMap<K, ()>,
}

impl<K> Default for IdSet<K> {
    fn default() -> Self {
        Self {
            map: IdMap::default(),
        }
    }
}

impl<K: Ord> IdSet<K> {
    /// Returns whether the key was new.
    pub(crate) fn insert(&mut self, key: K) -> bool {
        self.map.insert(key, ()).is_none()
    }

    pub(crate) fn contains<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.map.contains_key(key)
    }

    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}

impl<K: Ord> FromIterator<K> for IdSet<K> {
    fn from_iter<I: IntoIterator<Item = K>>(items: I) -> Self {
        Self {
            map: items.into_iter().map(|key| (key, ())).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IdMap, IdSet};

    #[test]
    fn a_later_insert_replaces_the_value_and_lookups_borrow() {
        let mut map: IdMap<String, i64> = IdMap::default();
        assert_eq!(map.insert("TASK-2".into(), 1), None);
        assert_eq!(map.insert("TASK-1".into(), 2), None);
        assert_eq!(map.insert("TASK-2".into(), 3), Some(1));
        assert_eq!(map.get("TASK-2"), Some(&3));
        assert_eq!(map.values().copied().collect::<Vec<_>>(), [2, 3]);
        *map.get_or_default("TASK-3".into()) += 5;
        assert_eq!(map.remove("TASK-3"), Some(5));
        assert!(!map.contains_key("TASK-3"));
        let collected: IdMap<&str, i64> = [("a", 1), ("a", 2)].into_iter().collect();
        assert_eq!(collected.get("a"), Some(&2));
        let mut set: IdSet<&str> = IdSet::default();
        assert!(set.insert("a"));
        assert!(!set.insert("a"));
        assert!(set.contains("a") && set.len() == 1);
    }
}
