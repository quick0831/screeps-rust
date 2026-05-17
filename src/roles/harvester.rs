use screeps::Creep;
use screeps::HasPosition;
use screeps::ObjectId;
use screeps::ResourceType;
use screeps::SharedCreepProperties;
use screeps::Source;
use screeps::StructureType;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::TransferErrorCode;
use screeps::find;
use serde::{Deserialize, Serialize};

use crate::SharedData;
use crate::utils::sort_unstable_by_distance;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HarvesterMemory {
    target: Option<ObjectId<Source>>,
    harvesting: bool,
}

impl HarvesterMemory {
    pub fn get_target(&self) -> Option<ObjectId<Source>> {
        self.target
    }
}

pub fn run(creep: &Creep, memory: &mut HarvesterMemory, d: &SharedData) {
    memory.target = d.source_alloc.delegate(creep).or(memory.target);
    let Some(target) = memory.target else { return };
    let Some(target) = target.resolve() else {
        return;
    };

    if creep.store().get_free_capacity(None) == 0 {
        memory.harvesting = false;
    }
    if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
        memory.harvesting = true;
    }

    if memory.harvesting {
        if let Err(HarvestErrorCode::NotInRange) = creep.harvest(&target) {
            let _ = creep.move_to(&target);
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
