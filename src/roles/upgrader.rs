use screeps::Creep;
use screeps::ResourceType;
use screeps::action_error_codes::UpgradeControllerErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::path_finder::PathFinder;
use crate::roles::RoleTrait;
use crate::room::RoomMemory;
use crate::room::SharedData;
use crate::transport_alloc::ImportTaskHandle;
use crate::transport_alloc::Task;
use crate::transport_alloc::TransportAllocation;
use crate::transport_alloc::TransportAllocator;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Upgrader {
    upgrading: bool,
    task: Option<Task>,
    #[serde(skip)]
    task_handle: Option<ImportTaskHandle>,
}

impl RoleTrait for Upgrader {
    fn register(&mut self, creep: &Creep, _d: &mut SharedData, transport: &mut TransportAllocator) {
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
            self.task_handle = Some(transport.register_creep_import(creep, self.task));
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

        if self.upgrading {
            let controller = creep.room().unwrap().controller().unwrap();
            if let Err(UpgradeControllerErrorCode::NotInRange) =
                creep.upgrade_controller(&controller)
            {
                path_finder.move_to(creep, &controller, 3);
            }
        } else if let Some(Task::Pickup(id)) = self.task {
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
        } else {
            // No task available, enter idle state
            path_finder.move_away_multi(creep, &d.keepouts);
            self.task = None;
        }
    }
}
