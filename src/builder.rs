use screeps::{
    Creep, ResourceType,
    action_error_codes::{BuildErrorCode, HarvestErrorCode},
    find,
};
use serde::{Deserialize, Serialize};

use crate::SharedData;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuilderMemory {
    building: bool,
}

pub fn run(creep: &Creep, memory: &mut BuilderMemory, _d: &SharedData) {
    if memory.building && creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
        memory.building = false;
        let _ = creep.say("🔄 harvest", false);
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
    } else {
        let sources = creep.room().unwrap().find(find::SOURCES, None);
        if let Err(HarvestErrorCode::NotInRange) = creep.harvest(&sources[0]) {
            let _ = creep.move_to(&sources[0]);
        }
    }
}
