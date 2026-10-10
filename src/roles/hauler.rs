use screeps::Creep;
use screeps::ResourceType;
use screeps::action_error_codes::TransferErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::roles::RoleTrait;
use crate::room::RoomMemory;
use crate::room::SharedData;
use crate::transport_alloc::Task;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Hauler {
    task: Option<Task>,
}

impl RoleTrait for Hauler {
    fn register(&mut self, creep: &Creep, d: &mut SharedData) {
        if creep.ticks_to_live().is_some_and(|ttl| ttl < 50) {
            if creep.store().get_used_capacity(None) == 0 {
                let _ = creep.suicide();
            } else {
                d.transport_alloc.register_creep_export(creep);
            }
        } else {
            d.transport_alloc.register_hauler(creep, self.task);
        }
    }

    fn run(&mut self, creep: &Creep, d: &mut SharedData, _room_memory: &mut RoomMemory) {
        if self.task.is_none() {
            self.task = d.transport_alloc.delegate(creep);
        }

        match self.task {
            Some(Task::Pickup(id)) => {
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
            }
            Some(Task::Supply(id)) => {
                if let Some(target) = id.resolve() {
                    let err = creep.transfer(&target, ResourceType::Energy, None);
                    if let Err(TransferErrorCode::NotInRange) = err {
                        d.path_finder.move_to(creep, target.pos(), 1);
                    } else {
                        self.task = None;
                    }
                } else {
                    // target failed to resolve, give up task
                    self.task = None;
                }
            }
            None => {
                // No task available, enter idle state
                d.path_finder.move_away_multi(creep, &d.keepouts);
            }
        }
    }
}
