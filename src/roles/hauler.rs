use screeps::Creep;
use screeps::ResourceType;
use screeps::StructureType;
use screeps::action_error_codes::TransferErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::find;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::SharedData;
use crate::utils::sort_unstable_by_distance;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HaulerMemory {
    carrying: bool,
}

pub fn run(creep: &Creep, memory: &mut HaulerMemory, _d: &SharedData) {
    if creep.store().get_free_capacity(None) == 0 {
        memory.carrying = true;
    }
    if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
        memory.carrying = false;
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
    } else {
        let structures = creep.room().unwrap().find(find::STRUCTURES, None);
        let mut targets = structures
            .into_iter()
            .filter(|s| matches!(s.structure_type(), StructureType::Container))
            .filter(|s| {
                s.as_has_store()
                    .and_then(|s| s.store().get(ResourceType::Energy))
                    .unwrap_or(0)
                    > 0
            })
            .collect::<Vec<_>>();
        targets = sort_unstable_by_distance(creep.pos(), targets);
        if let Some(target) = targets.first()
            && let Some(withdrawable) = target.as_withdrawable()
        {
            let err = creep.withdraw(withdrawable, ResourceType::Energy, None);
            if let Err(WithdrawErrorCode::NotInRange) = err {
                let _ = creep.move_to(target.clone());
            }
        }
    }
}
