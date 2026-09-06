use screeps::Creep;
use screeps::ResourceType;
use screeps::Room;
use screeps::Source;
use screeps::StructureContainer;
use screeps::StructureSpawn;
use screeps::StructureTower;
use screeps::StructureType;
use screeps::TextAlign;
use screeps::TextStyle;
use screeps::find;
use screeps::game;
use screeps::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde_wasm_bindgen::from_value;
use serde_wasm_bindgen::to_value;
use strum::IntoDiscriminant as _;

use crate::container::put_containers;
use crate::metric::Metric;
use crate::roles::*;
use crate::source_alloc::SourceAllocator;
use crate::spawn::process_spawning;
use crate::tower;
use crate::transport_alloc::EnergyStore;
use crate::transport_alloc::TransportAllocator;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RoomMemory {
    pub energy_rate: Metric,
}

pub struct SharedData {
    pub spawn: StructureSpawn,
    pub room: Room,
    pub sources: Vec<Source>,
    pub source_alloc: SourceAllocator,
    pub transport_alloc: TransportAllocator,
    pub role_count: RoleCount,
    pub energy: EnergyStatus,
}

#[derive(Debug, Default)]
pub struct RoleCount {
    pub haulers: usize,
    pub harvesters: usize,
    pub upgraders: usize,
    pub builders: usize,
}

#[derive(Debug, Default)]
pub struct EnergyStatus {
    pub available: u32,
    pub capacity: u32,
}

pub fn process_room(room: Room, spawns: Vec<StructureSpawn>, time: u32) {
    let mut room_memory: RoomMemory = from_value(room.memory()).unwrap_or_default();
    let spawn = spawns[0].clone();
    let sources = room.find(find::SOURCES, None);
    let source_alloc = SourceAllocator::new(&sources);
    let transport_alloc = TransportAllocator::new();
    let role_count = RoleCount::default();
    let energy = EnergyStatus {
        available: room.energy_available(),
        capacity: room.energy_capacity_available(),
    };

    let mut d = SharedData {
        spawn,
        room,
        sources,
        source_alloc,
        transport_alloc,
        role_count,
        energy,
    };

    put_containers(&d);

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

    for (_, memory) in &creep_mems {
        match memory.discriminant() {
            RoleType::Hauler => d.role_count.haulers += 1,
            RoleType::Harvester => d.role_count.harvesters += 1,
            RoleType::Upgrader => d.role_count.upgraders += 1,
            RoleType::Builder => d.role_count.builders += 1,
        }
    }

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
    d.source_alloc.allocate();
    d.transport_alloc.allocate();

    // Execute stage
    for (creep, mut memory) in creep_mems {
        memory.run(&creep, &d, &mut room_memory);

        creep.set_memory(&to_value(&memory).expect("Failed to serialize memory"));
    }

    process_spawning(&d);

    room_memory.energy_rate.record_finish();
    let stat_energy_rate = room_memory.energy_rate.calculate_output();

    d.room
        .set_memory(&to_value(&room_memory).expect("Failed to serialize Room memory"));

    let controller = d.room.controller().unwrap();
    let rcl = controller.level();
    let rcl_progress = controller.progress().unwrap_or(0);
    let rcl_progress_total = controller.progress_total().unwrap_or(0);
    let rcl_ratio = rcl_progress as f32 * 100. / rcl_progress_total as f32;

    let texts: [String; _] = [
        format!("Time: {time}"),
        format!("Energy: {} / {}", d.energy.available, d.energy.capacity),
        format!("RCL {rcl}: {rcl_progress} / {rcl_progress_total} ({rcl_ratio:.2}%)"),
        format!("Haulers: {}", d.role_count.haulers),
        format!("Harvesters: {}", d.role_count.harvesters),
        format!("Upgraders: {}", d.role_count.upgraders),
        format!("Builders: {}", d.role_count.builders),
        format!("Energy rate: {:.3}", stat_energy_rate),
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
}
