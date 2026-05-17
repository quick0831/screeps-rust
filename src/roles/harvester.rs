use screeps::Creep;
use screeps::HasPosition;
use screeps::ResourceType;
use screeps::SharedCreepProperties;
use screeps::StructureType;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::TransferErrorCode;
use screeps::find;
use serde::{Deserialize, Serialize};

use crate::SharedData;
use crate::utils::sort_unstable_by_distance;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HarvesterMemory {}

pub fn run(creep: &Creep, _memory: &mut HarvesterMemory, _d: &SharedData) {
    if creep.store().get_free_capacity(None) > 0 {
        let sources = creep.room().unwrap().find(find::SOURCES, None);
        if let Err(HarvestErrorCode::NotInRange) = creep.harvest(&sources[0]) {
            let _ = creep.move_to(&sources[0]);
        }
    } else {
        let structures = creep.room().unwrap().find(find::MY_STRUCTURES, None);
        let mut targets = structures
            .into_iter()
            .filter(|s| {
                matches!(
                    s.structure_type(),
                    StructureType::Extension | StructureType::Spawn | StructureType::Tower
                )
            })
            .filter(|s| {
                s.as_has_store()
                    .map(|s| s.store().get_free_capacity(Some(ResourceType::Energy)))
                    .unwrap_or(0)
                    > 0
            })
            .collect::<Vec<_>>();
        targets = sort_unstable_by_distance(creep.pos(), targets);
        if let Some(target) = targets.first()
            && let Some(transferable) = target.as_transferable()
        {
            let err = creep.transfer(transferable, ResourceType::Energy, None);
            if let Err(TransferErrorCode::NotInRange) = err {
                let _ = creep.move_to(target.clone());
            }
        }
    }
}
