use screeps::Creep;
use screeps::ResourceType;
use screeps::action_error_codes::TransferErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::path_finder::PathFinder;
use crate::roles::RoleTrait;
use crate::room::RoomMemory;
use crate::room::SharedData;
use crate::transport_alloc::HaulerTaskHandle;
use crate::transport_alloc::Task;
use crate::transport_alloc::TransportAllocation;
use crate::transport_alloc::TransportAllocator;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Hauler {
    task: Option<Task>,
    #[serde(skip)]
    task_handle: Option<HaulerTaskHandle>,
}

impl RoleTrait for Hauler {
    fn register(&mut self, creep: &Creep, _d: &mut SharedData, transport: &mut TransportAllocator) {
        if creep.ticks_to_live().is_some_and(|ttl| ttl < 50) {
            if creep.store().get_used_capacity(None) == 0 {
                let _ = creep.suicide();
            } else {
                // self.task_handle = Some(transport.register_creep_export(creep));
            }
        } else {
            self.task_handle = Some(transport.register_hauler(creep, self.task));
        }
    }

    fn run(
        &mut self,
        creep: &Creep,
        d: &mut SharedData,
        _room_memory: &mut RoomMemory,
        path_finder: &mut PathFinder,
        transport: &TransportAllocation,
    ) {
        if self.task.is_none()
            && let Some(handle) = &self.task_handle
        {
            self.task = handle.resolve(transport);
        }

        match self.task {
            Some(Task::Pickup(id)) => {
                if let Some(target) = id.resolve() {
                    let err = creep.withdraw(&target, ResourceType::Energy, None);
                    if let Err(WithdrawErrorCode::NotInRange) = err {
                        path_finder.move_to(creep, target.pos(), 1);
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
                        path_finder.move_to(creep, target.pos(), 1);
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
                path_finder.move_away_multi(creep, &d.keepouts);
            }
        }
    }
}
