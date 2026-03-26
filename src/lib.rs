use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};

use screeps::Part;
use screeps::SpawnOptions;
use screeps::StructureSpawn;
use screeps::game;

mod harvester;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Roles {
    Harvester,
}

#[derive(Serialize, Deserialize)]
struct CreepMem {
    role: Roles,
}

struct SharedData {
    spawn: StructureSpawn,
}

#[unsafe(export_name = "loop")]
pub extern "C" fn func_loop() {
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
        }
    }

    if creeps.keys().count() < 2 {
        let body = vec![Part::Move, Part::Work, Part::Carry];
        let name = format!("Harvester{time}");
        let mem = CreepMem {
            role: Roles::Harvester,
        };
        let option = SpawnOptions::new().memory(to_value(&mem).unwrap());
        let _ = d.spawn.spawn_creep_with_options(&body, &name, &option);
    }
}
