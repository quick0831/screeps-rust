use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};

use screeps::Creep;
use screeps::Part;
use screeps::SpawnOptions;
use screeps::StructureSpawn;
use screeps::game;

mod logging;

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

mod harvester;
mod upgrader;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Roles {
    Harvester,
    Upgrader,
}

#[derive(Serialize, Deserialize)]
struct CreepMem {
    role: Roles,
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
        let Ok(memory) = from_value::<CreepMem>(creep.memory()) else {
            continue;
        };
        match memory.role {
            Roles::Harvester => harvester::run(&creep, &memory, &d),
            Roles::Upgrader => upgrader::run(&creep, &memory, &d),
        }
    }

    if creeps
        .values()
        .filter(|c| is_role(c, Roles::Harvester))
        .count()
        < 2
    {
        let body = vec![Part::Move, Part::Work, Part::Carry];
        let name = format!("Harvester{time}");
        let mem = CreepMem {
            role: Roles::Harvester,
        };
        let option = SpawnOptions::new().memory(to_value(&mem).unwrap());
        let _ = d.spawn.spawn_creep_with_options(&body, &name, &option);
    }

    if creeps
        .values()
        .filter(|c| is_role(c, Roles::Upgrader))
        .count()
        < 2
    {
        let body = vec![Part::Move, Part::Work, Part::Carry];
        let name = format!("Upgrader{time}");
        let mem = CreepMem {
            role: Roles::Upgrader,
        };
        let option = SpawnOptions::new().memory(to_value(&mem).unwrap());
        let _ = d.spawn.spawn_creep_with_options(&body, &name, &option);
    }
}

fn is_role(creep: &Creep, role: Roles) -> bool {
    let Ok(memory) = from_value::<CreepMem>(creep.memory()) else {
        return false;
    };
    memory.role == role
}
