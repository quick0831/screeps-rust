use std::cmp::Ordering;
use std::cmp::max;

use screeps::Position;
use screeps::prelude::*;

pub fn diagonal_distance(a: Position, b: Position) -> u8 {
    let (ax, ay) = a.coords();
    let (bx, by) = b.coords();
    max(ax.abs_diff(bx), ay.abs_diff(by))
}

pub fn sort_unstable_by_distance<T: HasPosition>(center: Position, mut sites: Vec<T>) -> Vec<T> {
    let (cx, cy) = center.coords_signed();
    sites.sort_unstable_by_key(|s| {
        let (sx, sy) = s.pos().coords_signed();
        ((sx - cx) as i16).pow(2) as u16 + ((sy - cy) as i16).pow(2) as u16
    });
    sites
}

/// Associates an un-compared value with a compared key, intended for use in `BinaryHeap`
pub struct KeyCmp<K: Ord, V> {
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
