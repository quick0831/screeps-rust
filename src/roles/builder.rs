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

use crate::SharedData;
use crate::path_finder::path_away_from;
use crate::roles::RoleTrait;
use crate::utils::sort_unstable_by_distance;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Builder {
    target: Option<ObjectId<ConstructionSite>>,
    building: bool,
    fetch: bool,
}
impl RoleTrait for Builder {
    fn register(&self, _creep: &Creep, _d: &mut SharedData) {}

    fn run(&mut self, creep: &Creep, d: &SharedData) {
        if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
            self.building = false;
            let energy_avail = d.room.energy_available();
            let energy_cap = d.room.energy_capacity_available();
            self.fetch = energy_avail > 250 && energy_cap - energy_avail < 300;
            let msg = if self.fetch { "🫳 fetch" } else { "⏸️" };
            let _ = creep.say(msg, false);
        }
        if !self.building && creep.store().get_free_capacity(None) == 0 {
            self.building = true;
            self.target = None;
            let _ = creep.say("🚧 build", false);
        }

        if self.building {
            if let Some(target) = self.target
                && let Some(target) = target.resolve()
            {
                if let Err(BuildErrorCode::NotInRange) = creep.build(&target) {
                    let _ = creep.move_to(target);
                }
            } else {
                let construction_sites = d.room.find(find::MY_CONSTRUCTION_SITES, None);
                self.target = sort_unstable_by_distance(creep.pos(), construction_sites)
                    .first()
                    .and_then(|site| site.try_id());
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
