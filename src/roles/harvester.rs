use screeps::ConstructionSite;
use screeps::Creep;
use screeps::ObjectId;
use screeps::ResourceType;
use screeps::Source;
use screeps::StructureContainer;
use screeps::StructureObject;
use screeps::action_error_codes::BuildErrorCode;
use screeps::action_error_codes::CreepRepairErrorCode;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::TransferErrorCode;
use screeps::find;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::roles::RoleTrait;
use crate::room::RoomMemory;
use crate::room::SharedData;
use crate::utils::diagonal_distance;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Harvester {
    container: Option<ObjectId<StructureContainer>>,
    construction_site: Option<ObjectId<ConstructionSite>>,
    target: Option<ObjectId<Source>>,
    state: HarvesterState,
    record_haravest: Option<u32>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HarvesterState {
    Repair,
    Deposit,
    Build,
    DepositSpawn,
    #[default]
    #[serde(other)]
    Harvest,
}

impl RoleTrait for Harvester {
    fn register(&self, creep: &Creep, d: &mut SharedData) {
        d.source_alloc.register_harvester(creep, self.target);
    }

    fn run(&mut self, creep: &Creep, d: &SharedData, room_memory: &mut RoomMemory) {
        self.target = d.source_alloc.delegate(creep).or(self.target);
        let Some(target) = self.target else { return };
        let Some(target) = target.resolve() else {
            return;
        };

        if let Some(energy_before) = self.record_haravest.take() {
            let energy_after = creep.store().get(ResourceType::Energy).unwrap_or(0);
            room_memory
                .energy_rate
                .record_add(energy_before as i32 - energy_after as i32);
        }

        if self.state == HarvesterState::Harvest && creep.store().get_free_capacity(None) == 0 {
            let container = d
                .room
                .find(find::STRUCTURES, None)
                .into_iter()
                .filter_map(|s| match s {
                    StructureObject::StructureContainer(c) => Some(c),
                    _ => None,
                })
                .filter(|c| diagonal_distance(creep.pos(), c.pos()) <= 2)
                .find(|c| c.store().get_free_capacity(Some(ResourceType::Energy)) > 0);

            if let Some(container) = container {
                self.container = Some(container.id());
                if (container.hits() as f32 / container.hits_max() as f32) < 0.4 {
                    self.state = HarvesterState::Repair;
                } else {
                    self.state = HarvesterState::Deposit;
                }
            } else {
                self.container = None;
                self.construction_site = d
                    .room
                    .find(find::CONSTRUCTION_SITES, None)
                    .into_iter()
                    .find(|c| creep.pos().in_range_to(c.pos(), 2))
                    .and_then(|c| c.try_id());
                self.state = HarvesterState::Build;
            }
        }
        if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
            self.state = HarvesterState::Harvest;
        }

        match self.state {
            HarvesterState::Harvest => {
                if let Err(HarvestErrorCode::NotInRange) = creep.harvest(&target) {
                    let _ = creep.move_to(&target);
                }
            }
            HarvesterState::Repair => {
                let Some(container) = self.container.and_then(|id| id.resolve()) else {
                    self.state = HarvesterState::Build;
                    return;
                };
                let err = creep.repair(&container);
                if let Err(CreepRepairErrorCode::NotInRange) = err {
                    let _ = creep.move_to(&container);
                }
            }
            HarvesterState::Deposit => {
                let Some(container) = self.container.and_then(|id| id.resolve()) else {
                    self.state = HarvesterState::Build;
                    return;
                };
                let err = creep.transfer(&container, ResourceType::Energy, None);
                if let Err(TransferErrorCode::NotInRange) = err {
                    let _ = creep.move_to(&container);
                } else if err.is_ok() {
                    self.record_haravest =
                        Some(creep.store().get(ResourceType::Energy).unwrap_or(0));
                }
            }
            HarvesterState::Build => {
                let Some(site) = self.construction_site.and_then(|id| id.resolve()) else {
                    self.state = HarvesterState::Harvest;
                    return;
                };
                let err = creep.build(&site);
                if let Err(BuildErrorCode::NotInRange) = err {
                    let _ = creep.move_to(&site);
                }
            }
            HarvesterState::DepositSpawn => {
                let err = creep.transfer(&d.spawn, ResourceType::Energy, None);
                if let Err(TransferErrorCode::NotInRange) = err {
                    let _ = creep.move_to(&d.spawn);
                }
            }
        }
    }
}
