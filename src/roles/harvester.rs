use screeps::ConstructionSite;
use screeps::Creep;
use screeps::ObjectId;
use screeps::ResourceType;
use screeps::Source;
use screeps::StructureContainer;
use screeps::action_error_codes::BuildErrorCode;
use screeps::action_error_codes::CreepRepairErrorCode;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::TransferErrorCode;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::roles::RoleTrait;
use crate::room::RoomMemory;
use crate::room::SharedData;
use crate::source::ContainerInfo;

#[derive(Debug, Serialize, Deserialize)]
pub struct Harvester {
    #[serde(default)]
    container: Option<ObjectId<StructureContainer>>,
    #[serde(default)]
    construction_site: Option<ObjectId<ConstructionSite>>,
    target: ObjectId<Source>,
    #[serde(default)]
    state: HarvesterState,
    #[serde(default)]
    record_harvest: Option<u32>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HarvesterState {
    Repair,
    Build,
    Stall,
    #[default]
    #[serde(other)]
    Harvest,
}

impl Harvester {
    pub fn new(target: ObjectId<Source>) -> Self {
        Self {
            container: None,
            construction_site: None,
            target,
            state: HarvesterState::default(),
            record_harvest: None,
        }
    }
}

impl RoleTrait for Harvester {
    fn register(&self, creep: &Creep, d: &mut SharedData) {
        d.source_alloc.register_harvester(creep, self.target);
    }

    fn run(&mut self, creep: &Creep, d: &mut SharedData, room_memory: &mut RoomMemory) {
        let Some(target) = self.target.resolve() else {
            return;
        };

        if let Some(energy_before) = self.record_harvest.take() {
            let energy_after = creep.store().get(ResourceType::Energy).unwrap_or(0);
            room_memory
                .energy_rate
                .record_add(energy_after as i32 - energy_before as i32);
        }

        if self.state == HarvesterState::Stall
            || self.state == HarvesterState::Harvest && creep.store().get_free_capacity(None) == 0
        {
            let container = d
                .sources
                .iter()
                .find(|s| s.source.id() == self.target)
                .map(|s| s.container.clone())
                .unwrap_or(ContainerInfo::None);

            match container {
                ContainerInfo::Built(container_id) => {
                    let Some(container) = container_id.resolve() else {
                        return;
                    };
                    self.container = Some(container_id);
                    if (container.hits() as f32 / container.hits_max() as f32) < 0.4 {
                        self.state = HarvesterState::Repair;
                    } else {
                        // Deposit (because it can happen on the same tick as harvest)
                        let err = creep.transfer(&container, ResourceType::Energy, None);
                        if err.is_err() {
                            // avoid dropping resource on the ground
                            self.state = HarvesterState::Stall;
                        }
                        if let Err(TransferErrorCode::NotInRange) = err {
                            d.path_finder.move_to(creep, &container, 1);
                        }
                    }
                }
                ContainerInfo::Constructing(site_id) => {
                    self.container = None;
                    self.construction_site = Some(site_id);
                    self.state = HarvesterState::Build;
                }
                ContainerInfo::None => {}
            }
        }

        if creep.store().get(ResourceType::Energy).unwrap_or(0) == 0 {
            self.state = HarvesterState::Harvest;
        }

        match self.state {
            HarvesterState::Harvest => {
                let err = creep.harvest(&target);
                if let Err(HarvestErrorCode::NotInRange) = err {
                    d.path_finder.move_to(creep, &target, 1);
                } else if err.is_ok() {
                    self.record_harvest =
                        Some(creep.store().get(ResourceType::Energy).unwrap_or(0));
                }
            }
            HarvesterState::Repair => {
                let Some(container) = self.container.and_then(|id| id.resolve()) else {
                    self.state = HarvesterState::Build;
                    return;
                };
                let err = creep.repair(&container);
                if let Err(CreepRepairErrorCode::NotInRange) = err {
                    d.path_finder.move_to(creep, &container, 3);
                }
            }
            HarvesterState::Build => {
                let Some(site) = self.construction_site.and_then(|id| id.resolve()) else {
                    self.state = HarvesterState::Harvest;
                    return;
                };
                let err = creep.build(&site);
                if let Err(BuildErrorCode::NotInRange) = err {
                    d.path_finder.move_to(creep, &site, 3);
                }
            }
            HarvesterState::Stall => {}
        }
    }
}
