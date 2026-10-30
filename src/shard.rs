use std::collections::HashMap;

use screeps::RoomName;
use screeps::StructureSpawn;
use screeps::game;
use screeps::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use crate::roles::Role;
use crate::room;
use crate::room::RoomMemory;

#[derive(Default, Serialize, Deserialize)]
pub struct ShardMemory {
    pub room: HashMap<RoomName, RoomMemory>,
    pub creeps: HashMap<String, Role>,
}

impl ShardMemory {
    pub fn process(&mut self, time: u32) {
        let spawns: Vec<StructureSpawn> = game::spawns().values().collect();
        for room in game::rooms().values() {
            let spawns = spawns
                .iter()
                .filter(|&s| s.room().is_some_and(|r| r == room))
                .cloned()
                .collect();

            room::process_room(room, spawns, time, self);
        }
    }
}
