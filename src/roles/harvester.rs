use screeps::Creep;
use screeps::ObjectId;
use screeps::ResourceType;
use screeps::Source;
use screeps::StructureContainer;
use screeps::StructureObject;
use screeps::action_error_codes::CreepRepairErrorCode;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::TransferErrorCode;
use screeps::find;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::SharedData;
use crate::utils::diagonal_distance;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HarvesterMemory {
    container: Option<ObjectId<StructureContainer>>,
    target: Option<ObjectId<Source>>,
    state: HarvesterState,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HarvesterState {
    #[default]
    Harvest,
    Repair,
    Deposit,
    DepositSpawn,
}

pub fn register(creep: &Creep, memory: &HarvesterMemory, d: &mut SharedData) {
    d.source_alloc.register_harvester(creep, memory.target);
}

pub fn run(creep: &Creep, memory: &mut HarvesterMemory, d: &SharedData) {
    memory.target = d.source_alloc.delegate(creep).or(memory.target);
    let Some(target) = memory.target else { return };
    let Some(target) = target.resolve() else {
        return;
    };

    if memory.state == HarvesterState::Harvest && creep.store().get_free_capacity(None) == 0 {
        let container = d
            .room
            .find(find::STRUCTURES, None)
            .into_iter()
            .filter_map(|s| match s {
                StructureObject::StructureContainer(c) => Some(c),
                _ => None,
            })
            .filter(|c| diagonal_distance(creep.pos(), c.pos()) <= 2)
            .find(|c| c.store().get_free_capacity(Some(ResourceType::Energy)) > 0);

        if let Some(container) = container {
            memory.container = Some(container.id());
            if (container.hits() as f32 / container.hits_max() as f32) < 0.4 {
                memory.state = HarvesterState::Repair;
            } else {
                memory.state = HarvesterState::Deposit;
            }
        } else {
            memory.container = None;
            memory.state = HarvesterState::DepositSpawn;
        }
    }
    if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
        memory.state = HarvesterState::Harvest;
    }

    match memory.state {
        HarvesterState::Harvest => {
            if let Err(HarvestErrorCode::NotInRange) = creep.harvest(&target) {
                let _ = creep.move_to(&target);
            }
        }
        HarvesterState::Repair => {
            let Some(container) = memory.container.and_then(|id| id.resolve()) else {
                memory.state = HarvesterState::DepositSpawn;
                return;
            };
            let err = creep.repair(&container);
            if let Err(CreepRepairErrorCode::NotInRange) = err {
                let _ = creep.move_to(&container);
            }
        }
        HarvesterState::Deposit => {
            let Some(container) = memory.container.and_then(|id| id.resolve()) else {
                memory.state = HarvesterState::DepositSpawn;
                return;
            };
            let err = creep.transfer(&container, ResourceType::Energy, None);
            if let Err(TransferErrorCode::NotInRange) = err {
                let _ = creep.move_to(&container);
            }
        }
        HarvesterState::DepositSpawn => {
            let err = creep.transfer(&d.spawn, ResourceType::Energy, None);
            if let Err(TransferErrorCode::NotInRange) = err {
                let _ = creep.move_to(&d.spawn);
            }
        }
    }
}
