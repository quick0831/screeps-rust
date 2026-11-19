use screeps::Creep;
use screeps::HARVEST_POWER;
use screeps::Part;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::TransferErrorCode;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::creeps::RoleTrait;
use crate::path_finder::PathFinder;
use crate::room::RoomMemory;
use crate::room::SharedData;
use crate::transport_alloc::TransportAllocation;
use crate::transport_alloc::TransportAllocator;

#[derive(Debug, Serialize, Deserialize)]
pub struct MineralMiner {
    #[serde(default)]
    state: State,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Stall,
    #[default]
    #[serde(other)]
    Harvest,
}

impl MineralMiner {
    pub fn new() -> Self {
        Self {
            state: State::default(),
        }
    }
}

impl RoleTrait for MineralMiner {
    fn register(&mut self, creep: &Creep, d: &mut SharedData, _transport: &mut TransportAllocator) {
        d.mineral_manager.register_miner(creep);
    }

    fn run(
        &mut self,
        creep: &Creep,
        d: &mut SharedData,
        room_memory: &mut RoomMemory,
        path_finder: &mut PathFinder,
        _transport: &TransportAllocation,
    ) {
        let Some(container) = d.mineral_manager.get_container() else {
            return;
        };
        let mineral = d.mineral_manager.get_mineral();

        if self.state == State::Stall
            || self.state == State::Harvest && creep.store().get_free_capacity(None) == 0
        {
            // Deposit (because it can happen on the same tick as harvest)
            let resource_type = mineral.mineral_type();
            let err = creep.transfer(&container, resource_type, None);
            if err.is_err() {
                // avoid dropping resource on the ground
                self.state = State::Stall;
            }
            if let Err(TransferErrorCode::NotInRange) = err {
                path_finder.move_to(creep, &container, 1);
            }
        }

        if creep.store().get_used_capacity(None) == 0 {
            self.state = State::Harvest;
        }

        match self.state {
            State::Harvest => {
                let err = creep.harvest(&mineral);
                if let Err(HarvestErrorCode::NotInRange) = err {
                    path_finder.move_to(creep, &mineral, 1);
                } else if err.is_ok() {
                    let size = creep
                        .body()
                        .iter()
                        .filter(|p| p.part() == Part::Work)
                        .count();
                    room_memory
                        .energy_rate
                        .record_add(HARVEST_POWER as i32 * size as i32);
                }
            }
            State::Stall => {}
        }
    }
}
