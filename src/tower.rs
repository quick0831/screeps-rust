use screeps::HasPosition;
use screeps::StructureTower;
use screeps::find;

pub fn run(tower: StructureTower) {
    let room = tower.room().unwrap();

    let mut damaged_structures = room
        .find(find::STRUCTURES, None)
        .into_iter()
        .filter(|s| {
            if let Some(repairable) = s.as_repairable() {
                (repairable.hits() as f32 / repairable.hits_max() as f32) < 0.8
            } else {
                false
            }
        })
        .collect::<Vec<_>>();
    damaged_structures.sort_unstable_by_key(|s| {
        let (sx, sy) = s.pos().coords_signed();
        let (tx, ty) = tower.pos().coords_signed();
        (sx - tx).pow(2) + (sy - ty).pow(2)
    });

    if let Some(closest_damaged_structure) = damaged_structures.first()
        && let Some(structure) = closest_damaged_structure.as_repairable()
    {
        let _ = tower.repair(structure);
    }

    if let Some(closest_hostile) = tower.pos().find_closest_by_range(find::HOSTILE_CREEPS) {
        let _ = tower.attack(&closest_hostile);
    }
}
