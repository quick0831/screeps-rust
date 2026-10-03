use screeps::ConstructionSite;
use screeps::Creep;
use screeps::ObjectId;
use screeps::ResourceType;
use screeps::StructureType;
use screeps::action_error_codes::BuildErrorCode;
use screeps::action_error_codes::WithdrawErrorCode;
use screeps::find;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::roles::RoleTrait;
use crate::room::RoomMemory;
use crate::room::SharedData;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Builder {
    target: Option<ObjectId<ConstructionSite>>,
    state: BuilderState,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
enum BuilderState {
    Build,
    Fetch,
    #[default]
    Stall,
}

impl RoleTrait for Builder {
    fn register(&self, _creep: &Creep, _d: &mut SharedData) {}

    fn run(&mut self, creep: &Creep, d: &mut SharedData, _room_memory: &mut RoomMemory) {
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
            let _ = creep.say("🚧 build", false);
        }
        if creep.store().get_free_capacity(None) == 0 {
            self.state = BuilderState::Build;
        }

        match (self.state, self.target) {
            (BuilderState::Build, Some(target)) => {
                if let Some(target) = target.resolve() {
                    if let Err(BuildErrorCode::NotInRange) = creep.build(&target) {
                        d.path_finder.move_to(creep, target.pos(), 3);
                    }
                } else {
                    self.target = None;
                }
            }
            (BuilderState::Fetch, Some(_)) => {
                // grab energy from spawn and extensions
                let structures = creep.room().unwrap().find(find::MY_STRUCTURES, None);
                let center = creep.pos();
                let target = structures
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
                    .min_by_key(|s| center.get_range_to(s.pos()));
                if let Some(target) = target
                    && let Some(withdrawable) = target.as_withdrawable()
                {
                    let err = creep.withdraw(withdrawable, ResourceType::Energy, None);
                    if let Err(WithdrawErrorCode::NotInRange) = err {
                        d.path_finder.move_to(creep, target.pos(), 1);
                    }
                }
            }
            _ => {
                d.path_finder.move_away_multi(creep, &d.keepouts);
            }
        }
    }
}
