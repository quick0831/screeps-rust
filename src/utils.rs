use std::cmp::max;

use screeps::HasPosition;
use screeps::Position;

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
