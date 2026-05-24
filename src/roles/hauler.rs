use screeps::Creep;
use screeps::ResourceType;
use screeps::StructureType;
use screeps::action_error_codes::TransferErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::find;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::SharedData;
use crate::path_away::path_away_from;
use crate::roles::RoleTrait;
use crate::transport_alloc::EnergyStore;
use crate::transport_alloc::EnergyStoreId;
use crate::utils::sort_unstable_by_distance;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Hauler {
    target: Option<EnergyStoreId>,
    carrying: bool,
}

impl RoleTrait for Hauler {
    fn register(&self, creep: &Creep, d: &mut SharedData) {
        if !self.carrying {
            d.transport_alloc.register_hauler(creep, self.target);
        }
    }

    fn run(&mut self, creep: &Creep, d: &SharedData) {
        if !self.carrying {
            self.target = d.transport_alloc.delegate(creep).or(self.target);
        }

        if self.carrying {
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
                    s.as_has_store().map_or(0, |s| {
                        s.store().get_free_capacity(Some(ResourceType::Energy))
                    }) > 0
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
        } else if let Some(target) = self.target
            && let Some(target) = target.resolve()
        {
            if let EnergyStore::Creep(target_creep) = target {
                let err = target_creep.transfer(creep, ResourceType::Energy, None);
                if let Err(TransferErrorCode::NotInRange) = err {
                    let _ = creep.move_to(target_creep);
                } else {
                    self.target = None;
                }
            } else if let Some(withdrawable) = target.as_withdrawable() {
                let err = creep.withdraw(&withdrawable, ResourceType::Energy, None);
                if let Err(WithdrawErrorCode::NotInRange) = err {
                    let _ = creep.move_to(target.clone());
                } else {
                    self.target = None;
                }
            }
        } else {
            // move away from spawn
            let _ = path_away_from(creep, d.spawn.pos(), 7);
        }

        if creep.store().get_free_capacity(None) == 0 {
            self.carrying = true;
            self.target = None;
        }
        if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
            self.carrying = false;
        }
    }
}
