use screeps::Creep;
use screeps::ObjectId;
use screeps::ResourceType;
use screeps::StructureContainer;
use screeps::StructureObject;
use screeps::StructureType;
use screeps::action_error_codes::TransferErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::find;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::SharedData;
use crate::path_away::path_away_from;
use crate::utils::sort_unstable_by_distance;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HaulerMemory {
    target: Option<ObjectId<StructureContainer>>,
    carrying: bool,
}

pub fn run(creep: &Creep, memory: &mut HaulerMemory, d: &SharedData) {
    if creep.store().get_free_capacity(None) == 0 {
        memory.carrying = true;
        memory.target = None;
    }
    if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
        memory.carrying = false;

        if memory.target.is_none() {
            let structures = creep.room().unwrap().find(find::STRUCTURES, None);
            let mut targets = structures
                .into_iter()
                .filter_map(|s| {
                    if let StructureObject::StructureContainer(c) = s {
                        let energy = c.store().get(ResourceType::Energy).unwrap_or(0);
                        Some((c, energy))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            targets.sort_unstable_by_key(|(_, energy)| *energy);
            memory.target = targets.last().map(|(c, _)| c.id());
        }
    }

    if memory.carrying {
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
    } else if let Some(target) = memory.target
        && let Some(target) = target.resolve()
    {
        let err = creep.withdraw(&target, ResourceType::Energy, None);
        if let Err(WithdrawErrorCode::NotInRange) = err {
            let _ = creep.move_to(target.clone());
        }
    } else {
        // move away from spawn
        let _ = path_away_from(creep, d.spawn.pos(), 7);
    }
}
