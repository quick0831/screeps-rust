use screeps::HasPosition;
use screeps::Position;

pub fn sort_unstable_by_distance<T: HasPosition>(center: Position, mut sites: Vec<T>) -> Vec<T> {
    let (cx, cy) = center.coords_signed();
    sites.sort_unstable_by_key(|s| {
        let (sx, sy) = s.pos().coords_signed();
        (sx - cx).pow(2) + (sy - cy).pow(2)
    });
    sites
}
