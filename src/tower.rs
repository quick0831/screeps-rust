use screeps::StructureTower;
use screeps::StructureType;
use screeps::find;
use screeps::prelude::*;

pub fn run(tower: StructureTower) {
    let room = tower.room().unwrap();
    let center = tower.pos();

    let nearest_damaged_structures = room
        .find(find::STRUCTURES, None)
        .into_iter()
        .filter(|s| {
            if s.structure_type() == StructureType::Wall {
                // avoid fixing nearby walls (too hard to fill 300M hp)
                false
            } else if let Some(repairable) = s.as_repairable() {
                (repairable.hits() as f32 / repairable.hits_max() as f32) < 0.9
            } else {
                false
            }
        })
        .min_by_key(|s| center.get_range_to(s.pos()));

    if let Some(structure) = nearest_damaged_structures
        && let Some(repairable) = structure.as_repairable()
    {
        let _ = tower.repair(repairable);
    }

    if let Some(closest_hostile) = tower.pos().find_closest_by_range(find::HOSTILE_CREEPS) {
        let _ = tower.attack(&closest_hostile);
    }
}
