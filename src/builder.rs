use screeps::{
    Creep, ResourceType, SharedCreepProperties, StructureType,
    action_error_codes::{BuildErrorCode, WithdrawErrorCode},
    find,
};
use serde::{Deserialize, Serialize};

use crate::SharedData;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BuilderMemory {
    building: bool,
    fetch: bool,
}

pub fn run(creep: &Creep, memory: &mut BuilderMemory, d: &SharedData) {
    if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
        memory.building = false;
        let energy_avail = d.room.energy_available();
        let energy_cap = d.room.energy_capacity_available();
        memory.fetch = energy_avail >= 500 && energy_cap - energy_avail < 200;
        let msg = if memory.fetch { "🫳 fetch" } else { "⏸️" };
        let _ = creep.say(msg, false);
    }
    if !memory.building && creep.store().get_free_capacity(None) == 0 {
        memory.building = true;
        let _ = creep.say("🚧 build", false);
    }

    if memory.building {
        let construction_sites = creep.room().unwrap().find(find::CONSTRUCTION_SITES, None);
        if let Some(construction_site) = construction_sites.first()
            && let Err(BuildErrorCode::NotInRange) = creep.build(construction_site)
        {
            let _ = creep.move_to(construction_site);
        }
    } else if memory.fetch {
        // grab energy from spawn and extensions
        let structures = creep.room().unwrap().find(find::MY_STRUCTURES, None);
        let mut targets = structures.into_iter().filter(|s| {
            matches!(
                s.structure_type(),
                StructureType::Extension | StructureType::Spawn
            ) && s
                .as_has_store()
                .and_then(|s| s.store().get(ResourceType::Energy))
                .unwrap_or(0)
                > 0
        });
        if let Some(target) = targets.next()
            && let Some(withdrawable) = target.as_withdrawable()
        {
            let err = creep.withdraw(withdrawable, ResourceType::Energy, None);
            if let Err(WithdrawErrorCode::NotInRange) = err {
                let _ = creep.move_to(target.clone());
            }
        }
    }
}
