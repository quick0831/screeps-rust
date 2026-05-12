use log::info;
use log::warn;
use strum::IntoDiscriminant;
use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use strum::EnumDiscriminants;

use screeps::Part;
use screeps::SpawnOptions;
use screeps::StructureSpawn;
use screeps::game;

use crate::builder::BuilderMemory;
use crate::harvester::HarvesterMemory;
use crate::upgrader::UpgraderMemory;

mod logging;

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

mod builder;
mod harvester;
mod upgrader;

#[derive(Debug, Serialize, Deserialize, EnumDiscriminants)]
#[serde(rename_all = "snake_case", tag = "role")]
#[strum_discriminants(name(CreepRole))]
enum CreepMemory {
    Harvester(HarvesterMemory),
    Upgrader(UpgraderMemory),
    Builder(BuilderMemory),
}

struct SharedData {
    spawn: StructureSpawn,
}

#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    INIT_LOGGING.call_once(|| {
        // show all output of Info level, adjust as needed
        logging::setup_logging(logging::Info);
    });

    let time = game::time();

    let creeps = game::creeps();
    let d = SharedData {
        spawn: game::spawns().values().next().unwrap(),
    };

    for creep in creeps.values() {
        let Ok(memory) = from_value::<CreepMemory>(creep.memory()) else {
            continue;
        };
        match &memory {
            CreepMemory::Harvester(memory) => harvester::run(&creep, memory, &d),
            CreepMemory::Upgrader(memory) => upgrader::run(&creep, memory, &d),
            CreepMemory::Builder(memory) => builder::run(&creep, memory, &d),
        }
    }

    let roles: Vec<CreepRole> = creeps
        .values()
        .filter_map(|creep| {
            from_value::<CreepMemory>(creep.memory())
                .ok()
                .map(|mem| mem.discriminant())
        })
        .collect();

    if d.spawn.spawning().is_some() {
        // stop yapping
    } else if roles.iter().filter(|c| **c == CreepRole::Harvester).count() < 2 {
        let body = vec![Part::Move, Part::Work, Part::Carry];
        let name = format!("Harvester{time}");
        let mem = CreepMemory::Harvester(HarvesterMemory::default());
        let option = SpawnOptions::new().memory(to_value(&mem).unwrap());
        let _ = d.spawn.spawn_creep_with_options(&body, &name, &option);
        info!("Spawning: {name}");
    } else if roles.iter().filter(|c| **c == CreepRole::Upgrader).count() < 2 {
        let body = vec![Part::Move, Part::Work, Part::Carry];
        let name = format!("Upgrader{time}");
        let mem = CreepMemory::Upgrader(UpgraderMemory::default());
        let option = SpawnOptions::new().memory(to_value(&mem).unwrap());
        let _ = d.spawn.spawn_creep_with_options(&body, &name, &option);
        info!("Spawning: {name}");
    }

    let cpu_limit = game::cpu::limit();
    let cpu_usage = game::cpu::get_used();

    if cpu_usage.floor() as u32 > cpu_limit {
        warn!("Detect CPU spike: {cpu_usage:.2}/{cpu_limit}");
    }
}
