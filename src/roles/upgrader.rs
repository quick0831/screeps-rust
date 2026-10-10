use screeps::Creep;
use screeps::ResourceType;
use screeps::action_error_codes::UpgradeControllerErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::roles::RoleTrait;
use crate::room::RoomMemory;
use crate::room::SharedData;
use crate::transport_alloc::Task;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Upgrader {
    upgrading: bool,
    task: Option<Task>,
}

impl RoleTrait for Upgrader {
    fn register(&mut self, creep: &Creep, d: &mut SharedData) {
        if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
            self.upgrading = false;
            let _ = creep.say("🫳 fetch", false);
        }
        if !self.upgrading && creep.store().get_free_capacity(None) == 0 {
            self.upgrading = true;
            self.task = None;
            let _ = creep.say("⚡ upgrade", false);
        }
        if !self.upgrading {
            d.transport_alloc.register_creep_import(creep, self.task);
        }
    }

    fn run(&mut self, creep: &Creep, d: &mut SharedData, _room_memory: &mut RoomMemory) {
        if self.task.is_none() {
            self.task = d.transport_alloc.delegate(creep);
        }

        if self.upgrading {
            let controller = creep.room().unwrap().controller().unwrap();
            if let Err(UpgradeControllerErrorCode::NotInRange) =
                creep.upgrade_controller(&controller)
            {
                d.path_finder.move_to(creep, &controller, 3);
            }
        } else if let Some(Task::Pickup(id)) = self.task {
            if let Some(target) = id.resolve() {
                let err = creep.withdraw(&target, ResourceType::Energy, None);
                if let Err(WithdrawErrorCode::NotInRange) = err {
                    d.path_finder.move_to(creep, target.pos(), 1);
                } else {
                    self.task = None;
                }
            } else {
                // target failed to resolve, give up task
                self.task = None;
            }
        } else {
            // No task available, enter idle state
            d.path_finder.move_away_multi(creep, &d.keepouts);
            self.task = None;
        }
    }
}
