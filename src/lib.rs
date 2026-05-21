use std::cmp::max;
use std::cmp::min;
use std::collections::HashSet;

use js_sys::JsString;
use js_sys::Object;
use js_sys::Reflect;
use log::info;
use log::warn;
use screeps::Creep;
use screeps::Part;
use screeps::Room;
use screeps::SpawnOptions;
use screeps::StructureSpawn;
use screeps::StructureTower;
use screeps::StructureType;
use screeps::TextAlign;
use screeps::TextStyle;
use screeps::find;
use screeps::game;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use strum::EnumDiscriminants;
use strum::IntoDiscriminant;
use wasm_bindgen::prelude::*;

mod logging;
mod path_away;
mod roles;
mod source_alloc;
mod tower;
mod utils;

use crate::roles::builder::{self, BuilderMemory};
use crate::roles::harvester::{self, HarvesterMemory};
use crate::roles::hauler;
use crate::roles::hauler::HaulerMemory;
use crate::roles::upgrader::{self, UpgraderMemory};
use crate::source_alloc::SourceAllocator;

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

#[derive(Debug, Serialize, Deserialize, EnumDiscriminants)]
#[serde(rename_all = "snake_case", tag = "role")]
#[strum_discriminants(name(CreepRole))]
#[strum_discriminants(derive(strum::Display))]
enum CreepMemory {
    Hauler(HaulerMemory),
    Harvester(HarvesterMemory),
    Upgrader(UpgraderMemory),
    Builder(BuilderMemory),
}

struct SharedData {
    spawn: StructureSpawn,
    room: Room,
    source_alloc: SourceAllocator,
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
    let spawn = game::spawns().values().next().unwrap();
    let room = game::rooms().values().next().unwrap();
    let sources = room.find(find::SOURCES, None);
    let source_alloc = SourceAllocator::new(sources);
    let mut d = SharedData {
        spawn,
        room,
        source_alloc,
    };

    let towers = d
        .room
        .find(find::MY_STRUCTURES, None)
        .into_iter()
        .filter_map(|s| -> Option<StructureTower> { s.try_into().ok() });
    for tower in towers {
        tower::run(tower);
    }

    let creep_mems: Vec<(Creep, CreepMemory)> = creeps
        .values()
        .filter_map(|creep| from_value(creep.memory()).ok().map(|mem| (creep, mem)))
        .collect();

    // Register stage
    for (creep, memory) in &creep_mems {
        if let CreepMemory::Harvester(memory) = &memory {
            harvester::register(creep, memory, &mut d)
        }
    }

    // Allocation stage
    let harvester_spawn_size = d.source_alloc.allocate();

    // Execute stage
    for (creep, mut memory) in creep_mems {
        match &mut memory {
            CreepMemory::Hauler(memory) => hauler::run(&creep, memory, &d),
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
    let num_haulers = num_roles(CreepRole::Hauler);
    let num_harvesters = num_roles(CreepRole::Harvester);
    let num_upgraders = num_roles(CreepRole::Upgrader);
    let num_builders = num_roles(CreepRole::Builder);

    let has_construction_sites = !d.room.find(find::CONSTRUCTION_SITES, None).is_empty();

    let spawn_creep = |body: &[Part], name: &str, mem: &CreepMemory| {
        let option = SpawnOptions::new().memory(to_value(mem).unwrap());
        let result = d.spawn.spawn_creep_with_options(body, name, &option);
        info!("Spawning: {name}");
        result
    };

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
    } else if num_haulers < 2 && num_haulers < num_harvesters {
        let body = vec![Part::Move, Part::Move, Part::Carry, Part::Carry];
        let name = format!("Hauler{time}");
        let mem = CreepMemory::Hauler(HaulerMemory::default());
        let _ = spawn_creep(&body, &name, &mem);
    } else if harvester_spawn_size != 0 {
        let unit_part = [Part::Move, Part::Move, Part::Work, Part::Carry];
        let unit_cost: u32 = unit_part.map(Part::cost).into_iter().sum();
        let spawn_cap = (max(300, d.room.energy_capacity_available() - 300) / unit_cost) as u8;
        let spawn_size = min(harvester_spawn_size, spawn_cap) as usize;
        let body = unit_part.repeat(spawn_size);
        let name = format!("Harvester{time}");
        let mem = CreepMemory::Harvester(HarvesterMemory::default());
        let _ = spawn_creep(&body, &name, &mem);
    } else if num_upgraders < 3 {
        let body = vec![Part::Move, Part::Move, Part::Work, Part::Carry];
        let name = format!("Upgrader{time}");
        let mem = CreepMemory::Upgrader(UpgraderMemory::default());
        let _ = spawn_creep(&body, &name, &mem);
    } else if num_builders < 2 && has_construction_sites {
        let body = vec![Part::Move, Part::Move, Part::Work, Part::Carry];
        let name = format!("Builder{time}");
        let mem = CreepMemory::Builder(BuilderMemory::default());
        let _ = spawn_creep(&body, &name, &mem);
    }

    let controller = d.room.controller().unwrap();
    let rcl = controller.level();
    let rcl_progress = controller.progress().unwrap_or(0);
    let rcl_progress_total = controller.progress_total().unwrap_or(0);
    let rcl_ratio = rcl_progress as f32 * 100. / rcl_progress_total as f32;

    let texts: [String; _] = [
        format!("Time: {time}"),
        format!(
            "Energy: {} / {}",
            d.room.energy_available(),
            d.room.energy_capacity_available()
        ),
        format!("RCL {rcl}: {rcl_progress} / {rcl_progress_total} ({rcl_ratio:.2}%)"),
        format!("Haulers: {num_haulers}"),
        format!("Harvesters: {num_harvesters}"),
        format!("Upgraders: {num_upgraders}"),
        format!("Builders: {num_builders}"),
    ];

    let style = TextStyle::default().align(TextAlign::Left);
    let visual = d.room.visual();
    for (idx, text) in texts.into_iter().enumerate() {
        visual.text(0., (idx + 1) as f32, text, Some(style.clone()));
    }

    let pos = d.spawn.pos();
    let x = pos.x().u8();
    let y = pos.y().u8();
    for i in (x - 3)..=(x + 3) {
        for j in (y - 3)..=(y + 3) {
            let dist = i.abs_diff(x) + j.abs_diff(y);
            if dist == 0 {
                continue;
            }
            let ty = if dist == 1 || (i + j + x + y).is_multiple_of(2) {
                StructureType::Road
            } else {
                StructureType::Extension
            };
            let _ = d.room.create_construction_site(i, j, ty, None);
        }
    }

    let cpu_limit = game::cpu::limit();
    let cpu_usage = game::cpu::get_used();

    if cpu_usage.floor() as u32 > cpu_limit {
        warn!("Detect CPU spike: {cpu_usage:.2}/{cpu_limit}");
    }

    #[cfg(feature = "mmo")]
    if game::cpu::bucket() >= screeps::PIXEL_CPU_COST as i32 {
        match game::cpu::generate_pixel() {
            Ok(()) => info!("Generated pixel!"),
            Err(err) => warn!("Generate pixel failed: {err}"),
        }
    }
}
