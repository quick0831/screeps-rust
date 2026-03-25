use serde::Deserialize;
use serde::Serialize;
use serde_wasm_bindgen::to_value;

use screeps::Part;
use screeps::SpawnOptions;
use screeps::find;
use screeps::game;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Roles {
    Harvester,
}

#[derive(Serialize)]
struct CreepMem {
    role: Roles,
}

#[unsafe(export_name = "loop")]
pub extern "C" fn func_loop() {
    let time = game::time();

    let creeps = game::creeps();
    let spawn = game::spawns().values().next().unwrap();

    for creep in creeps.values() {
        let sources = creep.room().unwrap().find(find::SOURCES, None);
        let _ = creep.move_to(&sources[0]);
    }

    if creeps.keys().count() == 0 {
        let body = vec![Part::Move, Part::Work, Part::Carry];
        let name = format!("Harvester{time}");
        let mem = CreepMem {
            role: Roles::Harvester,
        };
        let option = SpawnOptions::new().memory(to_value(&mem).unwrap());
        let _ = spawn.spawn_creep_with_options(&body, &name, &option);
    }
}
