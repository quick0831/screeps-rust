use screeps::Creep;
use screeps::ObjectId;
use screeps::ResourceType;
use screeps::Source;
use screeps::StructureObject;
use screeps::StructureType;
use screeps::action_error_codes::CreepRepairErrorCode;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::TransferErrorCode;
use screeps::find;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::SharedData;
use crate::utils::diagonal_distance;
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
        let mut containers = d
            .room
            .find(find::STRUCTURES, None)
            .into_iter()
            .filter(|s| matches!(s.structure_type(), StructureType::Container))
            .filter(|s| diagonal_distance(creep.pos(), s.pos()) <= 2)
            .filter(|s| {
                s.as_has_store()
                    .map(|s| s.store().get_free_capacity(Some(ResourceType::Energy)))
                    .unwrap_or(0)
                    > 0
            })
            .collect::<Vec<_>>();
        if let Some(container) = containers.first()
            && let StructureObject::StructureContainer(container) = container
            && (container.hits() as f32 / container.hits_max() as f32) < 0.4
        {
            let err = creep.repair(container);
            if let Err(CreepRepairErrorCode::NotInRange) = err {
                let _ = creep.move_to(container.clone());
            }
            return;
        }
        let mut targets = d
            .room
            .find(find::MY_STRUCTURES, None)
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
        targets.append(&mut containers);
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
