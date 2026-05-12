use screeps::Creep;
use screeps::ResourceType;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::UpgradeControllerErrorCode;
use screeps::find;
use serde::{Deserialize, Serialize};

use crate::SharedData;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UpgraderMemory {
    upgrading: bool,
}

pub fn run(creep: &Creep, memory: &mut UpgraderMemory, _d: &SharedData) {
    if memory.upgrading && creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
        memory.upgrading = false;
        let _ = creep.say("🔄 harvest", false);
    }
    if !memory.upgrading && creep.store().get_free_capacity(None) == 0 {
        memory.upgrading = true;
        let _ = creep.say("⚡ upgrade", false);
    }

    if memory.upgrading {
        let controller = creep.room().unwrap().controller().unwrap();
        if let Err(UpgradeControllerErrorCode::NotInRange) = creep.upgrade_controller(&controller) {
            let _ = creep.move_to(controller);
        }
    } else {
        let sources = creep.room().unwrap().find(find::SOURCES, None);
        if let Err(HarvestErrorCode::NotInRange) = creep.harvest(&sources[0]) {
            let _ = creep.move_to(&sources[0]);
        }
    }
}
