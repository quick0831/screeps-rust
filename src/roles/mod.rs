use enum_dispatch::enum_dispatch;
use screeps::Creep;
use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

use crate::path_finder::PathFinder;
use crate::room::RoomMemory;
use crate::room::SharedData;
use crate::transport_alloc::TransportAllocation;
use crate::transport_alloc::TransportAllocator;

mod builder;
mod harvester;
mod hauler;
mod upgrader;

pub use builder::Builder;
pub use harvester::Harvester;
pub use hauler::Hauler;
pub use upgrader::Upgrader;

#[enum_dispatch]
pub trait RoleTrait {
    fn register(&mut self, creep: &Creep, d: &mut SharedData, transport: &mut TransportAllocator);
    fn run(
        &mut self,
        creep: &Creep,
        d: &mut SharedData,
        room_memory: &mut RoomMemory,
        path_finder: &mut PathFinder,
        transport: &TransportAllocation,
    );
}

#[enum_dispatch(RoleTrait)]
#[derive(Debug, Serialize, Deserialize, EnumDiscriminants)]
#[serde(rename_all = "snake_case", tag = "role")]
#[strum_discriminants(name(RoleType))]
#[strum_discriminants(derive(strum::Display))]
pub enum Role {
    Hauler,
    Harvester,
    Upgrader,
    Builder,
}
