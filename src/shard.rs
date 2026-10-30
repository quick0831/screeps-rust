use std::collections::HashMap;

use screeps::Creep;
use screeps::RoomName;
use screeps::StructureSpawn;
use screeps::game;
use screeps::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use crate::creeps::CreepMemory;
use crate::room;
use crate::room::RoomMemory;
use crate::spawn::NewCreepInfo;

#[derive(Default, Serialize, Deserialize)]
pub struct ShardMemory {
    pub room: HashMap<RoomName, RoomMemory>,
    pub creeps: HashMap<String, CreepMemory>,
}

impl ShardMemory {
    pub fn process(&mut self, time: u32) {
        let spawns: Vec<StructureSpawn> = game::spawns().values().collect();

        let creep_refs: HashMap<String, Creep> = game::creeps().entries().collect();
        let mut creep_mems: Vec<(Creep, &mut CreepMemory)> = self
            .creeps
            .iter_mut()
            .filter_map(|(name, mem)| {
                let creep = creep_refs.get(name)?.clone();
                Some((creep, mem))
            })
            .collect();

        let mut all_new_creeps = Vec::new();

        for room in game::rooms().values() {
            let room_memory: &mut RoomMemory = self.room.entry(room.name()).or_default();

            let spawns = spawns
                .iter()
                .filter(|&s| s.room().is_some_and(|r| r == room))
                .cloned()
                .collect();

            let mut new_creeps =
                room::process_room(room, room_memory, spawns, &mut creep_mems, time);

            all_new_creeps.append(&mut new_creeps);
        }

        for NewCreepInfo { name, mem } in all_new_creeps {
            self.creeps.insert(name, mem);
        }
    }
}
