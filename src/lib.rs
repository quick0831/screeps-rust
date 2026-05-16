use std::collections::HashSet;

use js_sys::JsString;
use js_sys::Object;
use js_sys::Reflect;
use log::info;
use log::warn;
use screeps::StructureTower;
use screeps::StructureType;
use screeps::TextAlign;
use screeps::TextStyle;
use screeps::find;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use strum::EnumDiscriminants;
use strum::IntoDiscriminant;
use wasm_bindgen::prelude::*;

use screeps::HasPosition;
use screeps::Part;
use screeps::Room;
use screeps::SpawnOptions;
use screeps::StructureSpawn;
use screeps::game;

use crate::builder::BuilderMemory;
use crate::harvester::HarvesterMemory;
use crate::upgrader::UpgraderMemory;

mod logging;
mod utils;

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

mod builder;
mod harvester;
mod tower;
mod upgrader;

#[derive(Debug, Serialize, Deserialize, EnumDiscriminants)]
#[serde(rename_all = "snake_case", tag = "role")]
#[strum_discriminants(name(CreepRole))]
#[strum_discriminants(derive(strum::Display))]
enum CreepMemory {
    Harvester(HarvesterMemory),
    Upgrader(UpgraderMemory),
    Builder(BuilderMemory),
}

struct SharedData {
    spawn: StructureSpawn,
    room: Room,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(thread_local_v2, js_name = Memory)]
    static MEMORY: Object;
}

#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    INIT_LOGGING.call_once(|| {
        // show all output of Info level, adjust as needed
        logging::setup_logging(logging::Info);
    });

    let time = game::time();

    // memory cleanup; memory gets created for all creeps upon spawning, and any time move_to
    // is used; this should be removed if you're using RawMemory/serde for persistence
    if time.is_multiple_of(1000) {
        info!("running memory cleanup");
        let mut alive_creeps = HashSet::new();
        // add all living creep names to a hashset
        for creep_name in game::creeps().keys() {
            alive_creeps.insert(creep_name);
        }

        // grab `Memory.creeps` (if it exists)
        MEMORY.with(|memory| {
            if let Ok(memory_creeps) = Reflect::get(memory, &JsString::from("creeps")) {
                // convert from JsValue to Object
                let memory_creeps: Object = memory_creeps.unchecked_into();
                // iterate memory creeps
                for creep_name_js in Object::keys(&memory_creeps).iter() {
                    // convert to String (after converting to JsString)
                    let creep_name = String::from(creep_name_js.dyn_ref::<JsString>().unwrap());

                    // check the HashSet for the creep name, deleting if not alive
                    if !alive_creeps.contains(&creep_name) {
                        info!("deleting memory for dead creep {}", creep_name);
                        let _ = Reflect::delete_property(&memory_creeps, &creep_name_js);
                    }
                }
            }
        });
    }

    let creeps = game::creeps();
    let d = SharedData {
        spawn: game::spawns().values().next().unwrap(),
        room: game::rooms().values().next().unwrap(),
    };

    let towers = d
        .room
        .find(find::MY_STRUCTURES, None)
        .into_iter()
        .filter_map(|s| -> Option<StructureTower> { s.try_into().ok() });
    for tower in towers {
        tower::run(tower);
    }

    for creep in creeps.values() {
        let Ok(mut memory) = from_value::<CreepMemory>(creep.memory()) else {
            continue;
        };
        match &mut memory {
            CreepMemory::Harvester(memory) => harvester::run(&creep, memory, &d),
            CreepMemory::Upgrader(memory) => upgrader::run(&creep, memory, &d),
            CreepMemory::Builder(memory) => builder::run(&creep, memory, &d),
        }

        creep.set_memory(&to_value(&memory).expect("Failed to serialize memory"));
    }

    let roles: Vec<CreepRole> = creeps
        .values()
        .filter_map(|creep| {
            from_value::<CreepMemory>(creep.memory())
                .ok()
                .map(|mem| mem.discriminant())
        })
        .collect();

    let num_roles = |role| roles.iter().filter(|c| **c == role).count();
    let num_harvesters = num_roles(CreepRole::Harvester);
    let num_upgraders = num_roles(CreepRole::Upgrader);
    let num_builders = num_roles(CreepRole::Builder);

    let has_construction_sites = !d.room.find(find::CONSTRUCTION_SITES, None).is_empty();

    if let Some(spawning) = d.spawn.spawning() {
        if let Some(name) = spawning.name().as_string()
            && let Some(creep) = game::creeps().get(name)
            && let Ok(memory) = from_value::<CreepMemory>(creep.memory())
        {
            let role = memory.discriminant();
            let pos = d.spawn.pos();
            let text = format!("🛠️ {role}");
            let style = TextStyle::default().align(TextAlign::Left);
            let visual = d.room.visual();
            visual.text(pos.x().u8() as f32, pos.y().u8() as f32, text, Some(style));
        }
    } else if num_harvesters < 4 {
        let body = vec![Part::Move, Part::Move, Part::Work, Part::Carry];
        let name = format!("Harvester{time}");
        let mem = CreepMemory::Harvester(HarvesterMemory::default());
        let option = SpawnOptions::new().memory(to_value(&mem).unwrap());
        let _ = d.spawn.spawn_creep_with_options(&body, &name, &option);
        info!("Spawning: {name}");
    } else if num_upgraders < 3 {
        let body = vec![Part::Move, Part::Move, Part::Work, Part::Carry];
        let name = format!("Upgrader{time}");
        let mem = CreepMemory::Upgrader(UpgraderMemory::default());
        let option = SpawnOptions::new().memory(to_value(&mem).unwrap());
        let _ = d.spawn.spawn_creep_with_options(&body, &name, &option);
        info!("Spawning: {name}");
    } else if num_builders < 2 && has_construction_sites {
        let body = vec![Part::Move, Part::Move, Part::Work, Part::Carry];
        let name = format!("Builder{time}");
        let mem = CreepMemory::Builder(BuilderMemory::default());
        let option = SpawnOptions::new().memory(to_value(&mem).unwrap());
        let _ = d.spawn.spawn_creep_with_options(&body, &name, &option);
        info!("Spawning: {name}");
    }

    let style = TextStyle::default().align(TextAlign::Left);
    let visual = d.room.visual();
    let text = format!("Time: {time}");
    visual.text(0., 1., text, Some(style.clone()));
    let text = format!("Harvesters: {num_harvesters}");
    visual.text(0., 2., text, Some(style.clone()));
    let text = format!("Upgraders: {num_upgraders}");
    visual.text(0., 3., text, Some(style.clone()));
    let text = format!("Builders: {num_builders}");
    visual.text(0., 4., text, Some(style.clone()));

    let pos = d.spawn.pos();
    let x = pos.x().u8();
    let y = pos.y().u8();
    for i in (x - 3)..=(x + 3) {
        for j in (y - 3)..=(y + 3) {
            let ty = if (i + j + x + y).is_multiple_of(2) {
                StructureType::Road
            } else {
                StructureType::Extension
            };
            if i.abs_diff(x) + j.abs_diff(y) > 1 {
                let _ = d.room.create_construction_site(i, j, ty, None);
            }
        }
    }

    let cpu_limit = game::cpu::limit();
    let cpu_usage = game::cpu::get_used();

    if cpu_usage.floor() as u32 > cpu_limit {
        warn!("Detect CPU spike: {cpu_usage:.2}/{cpu_limit}");
    }
}
