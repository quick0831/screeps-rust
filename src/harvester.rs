use screeps::Creep;
use screeps::SharedCreepProperties;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::TransferErrorCode;
use screeps::find;
use serde::{Deserialize, Serialize};

use crate::SharedData;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HarvesterMemory {}

pub fn run(creep: &Creep, _memory: &HarvesterMemory, d: &SharedData) {
    if creep.store().get_free_capacity(None) > 0 {
        let sources = creep.room().unwrap().find(find::SOURCES, None);
        if let Err(HarvestErrorCode::NotInRange) = creep.harvest(&sources[0]) {
            let _ = creep.move_to(&sources[0]);
        }
    } else {
        let err = creep.transfer(&d.spawn, screeps::ResourceType::Energy, None);
        if let Err(TransferErrorCode::NotInRange) = err {
            let _ = creep.move_to(d.spawn.clone());
        }
    }
}
