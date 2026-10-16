use std::cmp::min;

use screeps::ResourceType;
use screeps::StructureObject;
use screeps::StructureTower;
use screeps::TOWER_CAPACITY;
use screeps::find;
use screeps::prelude::*;

use crate::room::SharedData;
use crate::transport_alloc::Priority;
use crate::utils::Fraction;

const ENERGY_THRESHOLD: u32 = (TOWER_CAPACITY as f64 * 0.8) as u32;

pub fn run(tower: StructureTower, d: &mut SharedData) {
    let room = tower.room().unwrap();
    let center = tower.pos();

    let store = tower.store();
    let energy_available = store.get(ResourceType::Energy).unwrap_or(0);
    if energy_available < ENERGY_THRESHOLD {
        d.transport_alloc
            .register_import(tower.clone(), Priority::Low);
    }

    if let Some(closest_hostile) = center.find_closest_by_range(find::HOSTILE_CREEPS) {
        let _ = tower.attack(&closest_hostile);
        return;
    }

    let nearest_damaged_structures = room
        .find(find::STRUCTURES, None)
        .into_iter()
        .filter(|s| {
            if let StructureObject::StructureWall(_) = s {
                // avoid fixing nearby walls (too hard to fill 300M hp)
                false
            } else if let StructureObject::StructureRampart(s) = s {
                s.hits() < min(25_000_000, (s.hits_max() as f32 * 0.9) as u32)
            } else if let Some(repairable) = s.as_repairable() {
                Fraction(repairable.hits(), repairable.hits_max()) < Fraction(9, 10)
            } else {
                false
            }
        })
        .min_by_key(|s| {
            (
                s.as_repairable().and_then(|repairable| {
                    let hit_ratio = Fraction(repairable.hits(), repairable.hits_max());
                    if hit_ratio < Fraction(1, 20) {
                        Some(hit_ratio)
                    } else {
                        None
                    }
                }),
                center.get_range_to(s.pos()),
            )
        });

    if let Some(structure) = nearest_damaged_structures
        && let Some(repairable) = structure.as_repairable()
    {
        let _ = tower.repair(repairable);
    }
}
