use std::{cmp::Ordering, collections::BinaryHeap};

/// Associates an un-compared value with a compared key, intended for use in `BinaryHeap`
struct KeyCmp<K: Ord, V> {
    pub key: K,
    pub value: V,
}

impl<K: Ord, V> PartialEq for KeyCmp<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key.eq(&other.key)
    }
}

impl<K: Ord, V> Eq for KeyCmp<K, V> {}

impl<K: Ord, V> PartialOrd for KeyCmp<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Ord, V> Ord for KeyCmp<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

pub struct PriorityQueue<K: Ord, V> {
    heap: BinaryHeap<KeyCmp<K, V>>,
}

impl<K: Ord, V> FromIterator<(K, V)> for PriorityQueue<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let heap = iter
            .into_iter()
            .map(|(key, value)| KeyCmp { key, value })
            .collect();
        Self { heap }
    }
}

impl<K: Ord, V> PriorityQueue<K, V> {
    #[allow(unused)]
    pub fn push(&mut self, key: K, value: V) {
        self.heap.push(KeyCmp { key, value });
    }

    #[allow(unused)]
    pub fn pop(&mut self) -> Option<(K, V)> {
        let KeyCmp { key, value } = self.heap.pop()?;
        Some((key, value))
    }
}

impl<K: Ord, V> IntoIterator for PriorityQueue<K, V> {
    type Item = (K, V);
    type IntoIter = PqIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        PqIter(self.heap.into_iter())
    }
}

pub struct PqIter<K: Ord, V>(<BinaryHeap<KeyCmp<K, V>> as IntoIterator>::IntoIter);

impl<K: Ord, V> Iterator for PqIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let KeyCmp { key, value } = self.0.next()?;
        Some((key, value))
    }
}
