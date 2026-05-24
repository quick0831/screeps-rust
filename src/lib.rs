use std::cmp::max;
use std::cmp::min;

use log::error;
use log::info;
use log::warn;
use screeps::Creep;
use screeps::Part;
use screeps::ResourceType;
use screeps::Room;
use screeps::SpawnOptions;
use screeps::StructureContainer;
use screeps::StructureSpawn;
use screeps::StructureTower;
use screeps::StructureType;
use screeps::TextAlign;
use screeps::TextStyle;
use screeps::find;
use screeps::game;
use screeps::prelude::*;
use serde_wasm_bindgen::{from_value, to_value};
use strum::IntoDiscriminant;
use wasm_bindgen::prelude::*;

mod logging;
mod memory;
mod path_away;
mod roles;
mod source_alloc;
mod tower;
mod transport_alloc;
mod utils;

use crate::memory::cleanup_memory;
use crate::roles::*;
use crate::source_alloc::SourceAllocator;
use crate::transport_alloc::EnergyStore;
use crate::transport_alloc::TransportAllocator;

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

struct SharedData {
    spawn: StructureSpawn,
    room: Room,
    source_alloc: SourceAllocator,
    transport_alloc: TransportAllocator,
}

#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    INIT_LOGGING.call_once(|| {
        // show all output of Info level, adjust as needed
        logging::setup_logging(logging::Info);
    });

    let time = game::time();

    if time.is_multiple_of(100) {
        cleanup_memory();
    }

    let spawn = game::spawns().values().next().unwrap();
    let room = game::rooms().values().next().unwrap();
    let sources = room.find(find::SOURCES, None);
    let source_alloc = SourceAllocator::new(sources);
    let transport_alloc = TransportAllocator::new();
    let mut d = SharedData {
        spawn,
        room,
        source_alloc,
        transport_alloc,
    };

    let towers = d
        .room
        .find(find::MY_STRUCTURES, None)
        .into_iter()
        .filter_map(|s| -> Option<StructureTower> { s.try_into().ok() });
    for tower in towers {
        tower::run(tower);
    }

    let creep_mems: Vec<(Creep, Role)> = game::creeps()
        .values()
        .filter_map(|creep| from_value(creep.memory()).ok().map(|mem| (creep, mem)))
        .collect();

    let roles: Vec<RoleType> = creep_mems
        .iter()
        .map(|(_, memory)| memory.discriminant())
        .collect();

    let num_roles = |role| roles.iter().filter(|c| **c == role).count();
    let num_haulers = num_roles(RoleType::Hauler);
    let num_harvesters = num_roles(RoleType::Harvester);
    let num_upgraders = num_roles(RoleType::Upgrader);
    let num_builders = num_roles(RoleType::Builder);

    // Register stage
    for (creep, memory) in &creep_mems {
        memory.register(creep, &mut d);
    }

    for non_empty_container in d
        .room
        .find(find::STRUCTURES, None)
        .into_iter()
        .filter_map(|s| -> Option<StructureContainer> { s.try_into().ok() })
        .filter(|c| c.store().get(ResourceType::Energy).unwrap_or(0) > 0)
    {
        d.transport_alloc
            .file_request(EnergyStore::Container(non_empty_container));
    }

    // Allocation stage
    let harvester_spawn_size = d.source_alloc.allocate();
    d.transport_alloc.allocate();

    // Execute stage
    for (creep, mut memory) in creep_mems {
        memory.run(&creep, &d);

        creep.set_memory(&to_value(&memory).expect("Failed to serialize memory"));
    }

    let has_construction_sites = !d.room.find(find::CONSTRUCTION_SITES, None).is_empty();

    let spawn_creep = |body: &[Part], name: &str, mem: &Role| {
        let cost: u32 = body.iter().map(|p| p.cost()).sum();
        if d.room.energy_available() > cost {
            let option = SpawnOptions::new().memory(to_value(mem).unwrap());
            if let Err(err) = d.spawn.spawn_creep_with_options(body, name, &option) {
                error!("Spawning error: {err}");
            } else {
                info!("Spawning: {name}");
            }
        } else {
            info!("Spawning: Not enough energy!");
        }
    };

    if let Some(spawning) = d.spawn.spawning() {
        if let Some(name) = spawning.name().as_string()
            && let Some(creep) = game::creeps().get(name)
            && let Ok(memory) = from_value::<Role>(creep.memory())
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
        let mem = Hauler::default().into();
        spawn_creep(&body, &name, &mem);
    } else if harvester_spawn_size != 0 {
        let unit_part = [Part::Move, Part::Work, Part::Carry];
        let unit_cost: u32 = unit_part.map(Part::cost).into_iter().sum();
        let spawn_cap = (max(300, d.room.energy_capacity_available() - 300) / unit_cost) as u8;
        let spawn_size = min(harvester_spawn_size, spawn_cap) as usize;
        let body = unit_part.repeat(spawn_size);
        let name = format!("Harvester{time}");
        let mem = Harvester::default().into();
        spawn_creep(&body, &name, &mem);
    } else if num_upgraders < 3 {
        let body = vec![Part::Move, Part::Move, Part::Work, Part::Carry];
        let name = format!("Upgrader{time}");
        let mem = Upgrader::default().into();
        spawn_creep(&body, &name, &mem);
    } else if num_builders < 2 && has_construction_sites {
        let body = vec![Part::Move, Part::Move, Part::Work, Part::Carry];
        let name = format!("Builder{time}");
        let mem = Builder::default().into();
        spawn_creep(&body, &name, &mem);
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
