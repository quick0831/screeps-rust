use screeps::Creep;
use screeps::ResourceType;
use screeps::action_error_codes::BuildErrorCode;
use screeps::action_error_codes::CreepRepairErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::construction_alloc::ConstructionTask;
use crate::path_finder::PathFinder;
use crate::roles::RoleTrait;
use crate::room::RoomMemory;
use crate::room::SharedData;
use crate::transport_alloc::ImportTaskHandle;
use crate::transport_alloc::TransportAllocation;
use crate::transport_alloc::TransportAllocator;
use crate::transport_alloc::TransportTask;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Builder {
    work_task: Option<ConstructionTask>,
    fetch_task: Option<TransportTask>,
    state: BuilderState,
    #[serde(skip)]
    task_handle: Option<ImportTaskHandle>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum BuilderState {
    Work,
    #[default]
    #[serde(other)]
    Fetch,
}

impl RoleTrait for Builder {
    fn register(&mut self, creep: &Creep, d: &mut SharedData, transport: &mut TransportAllocator) {
        if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
            self.state = BuilderState::Fetch;
        }
        if creep.store().get_free_capacity(None) == 0 {
            self.state = BuilderState::Work;
        }
        if self.state == BuilderState::Fetch {
            self.task_handle = Some(transport.register_creep_import(creep, self.fetch_task));
        }

        d.construction_alloc.register_builder(creep, self.work_task);
    }

    fn run(
        &mut self,
        creep: &Creep,
        d: &mut SharedData,
        _room_memory: &mut RoomMemory,
        path_finder: &mut PathFinder,
        transport: &TransportAllocation,
    ) {
        if self.fetch_task.is_none()
            && let Some(handle) = &self.task_handle
        {
            self.fetch_task = handle.resolve(transport);
        }

        self.work_task = d.construction_alloc.delegate(creep);

        match (self.state, self.work_task, self.fetch_task) {
            (BuilderState::Work, Some(task), _) => match task {
                ConstructionTask::Build(id) => {
                    if let Some(target) = id.resolve() {
                        if let Err(BuildErrorCode::NotInRange) = creep.build(&target) {
                            path_finder.move_to(creep, target.pos(), 3);
                        }
                    } else {
                        self.fetch_task = None;
                    }
                }
                ConstructionTask::Repair(id) => {
                    if let Some(target) = id.resolve() {
                        if let Err(CreepRepairErrorCode::NotInRange) = creep.repair(&target) {
                            path_finder.move_to(creep, target.pos(), 3);
                        }
                    } else {
                        self.fetch_task = None;
                    }
                }
            },
            (BuilderState::Fetch, Some(_), Some(TransportTask::Pickup(id))) => {
                if let Some(target) = id.resolve() {
                    let err = creep.withdraw(&target, ResourceType::Energy, None);
                    if let Err(WithdrawErrorCode::NotInRange) = err {
                        path_finder.move_to(creep, target.pos(), 1);
                    } else {
                        self.fetch_task = None;
                    }
                } else {
                    // target failed to resolve, give up task
                    self.fetch_task = None;
                }
            }
            _ => {
                self.fetch_task = None;
            }
        }

        if self.work_task.is_none() && self.fetch_task.is_none() {
            path_finder.idle(creep);
        }
    }
}
