use screeps::ResourceType;
use screeps::Room;
use screeps::StructureObject;
use screeps::StructureTower;
use screeps::TOWER_CAPACITY;
use screeps::find;
use screeps::prelude::*;

use crate::transport_alloc::Priority;
use crate::transport_alloc::TransportAllocator;
use crate::utils::Fraction;

const ENERGY_THRESHOLD: u32 = (TOWER_CAPACITY as f64 * 0.8) as u32;

pub fn run(towers: &[StructureTower], room: &Room, transport: &mut TransportAllocator) {
    let damaged_structures: Vec<_> = room
        .find(find::STRUCTURES, None)
        .into_iter()
        .filter_map(|s| {
            if let StructureObject::StructureWall(_) = s {
                // avoid fixing nearby walls (too hard to fill 300M hp)
            } else if let StructureObject::StructureRampart(ref rampart) = s {
                // only repair a rampart up to 1000 hits
                let hits = rampart.hits();
                if hits < 1000 {
                    return Some((s, Fraction(hits, 5000)));
                }
            } else if let Some(repairable) = s.as_repairable() {
                // fix anything below 1/5 of hits
                let hit_ratio = Fraction(repairable.hits(), repairable.hits_max());
                if hit_ratio < Fraction(1, 5) {
                    return Some((s, hit_ratio));
                }
            }
            None
        })
        .collect();

    for tower in towers {
        let center = tower.pos();

        let store = tower.store();
        let energy_available = store.get(ResourceType::Energy).unwrap_or(0);
        if energy_available < ENERGY_THRESHOLD {
            transport.register_import(tower.clone(), Priority::Low);
        }

        if let Some(closest_hostile) = center.find_closest_by_range(find::HOSTILE_CREEPS) {
            let _ = tower.attack(&closest_hostile);
            return;
        }

        let nearest_damaged_structures = damaged_structures
            .iter()
            .min_by_key(|(_, hit_ratio)| *hit_ratio);

        if let Some((structure, _)) = nearest_damaged_structures
            && let Some(repairable) = structure.as_repairable()
        {
            let _ = tower.repair(repairable);
        }
    }
}
