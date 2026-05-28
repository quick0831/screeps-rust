use screeps::Creep;
use screeps::ResourceType;
use screeps::StructureType;
use screeps::action_error_codes::UpgradeControllerErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::find;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::RoomMemory;
use crate::SharedData;
use crate::path_finder::path_away_from;
use crate::roles::RoleTrait;
use crate::utils::sort_unstable_by_distance;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Upgrader {
    upgrading: bool,
    fetch: bool,
}

impl RoleTrait for Upgrader {
    fn register(&self, _creep: &Creep, _d: &mut SharedData) {}

    fn run(&mut self, creep: &Creep, d: &SharedData, _room_memory: &mut RoomMemory) {
        if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
            self.upgrading = false;
            let energy_avail = d.room.energy_available();
            let energy_cap = d.room.energy_capacity_available();
            self.fetch = energy_avail > 250 && energy_cap - energy_avail < 300;
            let msg = if self.fetch { "🫳 fetch" } else { "⏸️" };
            let _ = creep.say(msg, false);
        }
        if !self.upgrading && creep.store().get_free_capacity(None) == 0 {
            self.upgrading = true;
            let _ = creep.say("⚡ upgrade", false);
        }

        if self.upgrading {
            let controller = creep.room().unwrap().controller().unwrap();
            if let Err(UpgradeControllerErrorCode::NotInRange) =
                creep.upgrade_controller(&controller)
            {
                let _ = creep.move_to(controller);
            }
        } else if self.fetch {
            // grab energy from spawn and extensions
            let structures = creep.room().unwrap().find(find::MY_STRUCTURES, None);
            let mut targets = structures
                .into_iter()
                .filter(|s| {
                    matches!(
                        s.structure_type(),
                        StructureType::Extension | StructureType::Spawn
                    )
                })
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
        } else {
            // move away from spawn
            let _ = path_away_from(creep, d.spawn.pos(), 7);
        }
    }
}
