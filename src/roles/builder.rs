use screeps::ConstructionSite;
use screeps::Creep;
use screeps::ObjectId;
use screeps::ResourceType;
use screeps::action_error_codes::BuildErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::find;
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
pub struct Builder {
    target: Option<ObjectId<ConstructionSite>>,
    task: Option<Task>,
    state: BuilderState,
    #[serde(skip)]
    task_handle: Option<ImportTaskHandle>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum BuilderState {
    Build,
    Fetch,
    #[default]
    Stall,
}

impl RoleTrait for Builder {
    fn register(&mut self, creep: &Creep, d: &mut SharedData, transport: &mut TransportAllocator) {
        if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
            let energy_avail = d.room.energy_available();
            let energy_cap = d.room.energy_capacity_available();
            let msg;
            (self.state, msg) = if energy_avail > 250 && energy_cap - energy_avail < 300 {
                (BuilderState::Fetch, "🫳 fetch")
            } else {
                (BuilderState::Stall, "⏸️")
            };
            let _ = creep.say(msg, false);
        }
        if self.target.is_none() {
            let center = creep.pos();
            self.target = d
                .room
                .find(find::MY_CONSTRUCTION_SITES, None)
                .into_iter()
                .min_by_key(|s| center.get_range_to(s.pos()))
                .and_then(|site| site.try_id());
        }
        if creep.store().get_free_capacity(None) == 0 {
            self.state = BuilderState::Build;
            let _ = creep.say("🚧 build", false);
        }
        if self.state == BuilderState::Fetch {
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

        match (self.state, self.target, self.task) {
            (BuilderState::Build, Some(target), _) => {
                if let Some(target) = target.resolve() {
                    if let Err(BuildErrorCode::NotInRange) = creep.build(&target) {
                        path_finder.move_to(creep, target.pos(), 3);
                    }
                } else {
                    self.target = None;
                }
            }
            (BuilderState::Fetch, Some(_), Some(Task::Pickup(id))) => {
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
            _ => {
                path_finder.move_away_multi(creep, &d.keepouts);
                self.task = None;
            }
        }
    }
}
