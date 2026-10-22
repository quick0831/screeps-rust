use screeps::Creep;
use screeps::ResourceType;
use screeps::Room;
use screeps::StructureExtension;
use screeps::StructureSpawn;
use screeps::StructureStorage;
use screeps::StructureTower;
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

use crate::construction_alloc::ConstructionAllocator;
use crate::container::put_containers;
use crate::metric::Metric;
use crate::path_finder::PathFinder;
use crate::path_finder::SearchGoal;
use crate::roles::*;
use crate::source::ContainerInfo;
use crate::source::SourceInfo;
use crate::source::ananlyze_source;
use crate::source_alloc::SourceAllocator;
use crate::spawn::process_spawning;
use crate::tower;
use crate::transport_alloc::Priority;
use crate::transport_alloc::TransportAllocator;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RoomMemory {
    pub energy_rate: Metric,
}

pub struct SharedData {
    pub spawns: Vec<StructureSpawn>,
    pub room: Room,
    pub sources: Vec<SourceInfo>,
    pub source_alloc: SourceAllocator,
    pub construction_alloc: ConstructionAllocator,
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
    if spawns.is_empty() {
        return;
    }
    let sources: Vec<_> = room
        .find(find::SOURCES, None)
        .into_iter()
        .map(|s| ananlyze_source(s, &room))
        .collect();
    let source_alloc = SourceAllocator::new(&sources);
    let construction_sites = room.find(find::MY_CONSTRUCTION_SITES, None);
    let construction_alloc = ConstructionAllocator::new(construction_sites);
    let role_count = RoleCount::default();
    let energy = EnergyStatus {
        available: room.energy_available(),
        capacity: room.energy_capacity_available(),
    };

    let mut creep_mems: Vec<(Creep, Role)> = game::creeps()
        .values()
        .filter_map(|creep| from_value(creep.memory()).ok().map(|mem| (creep, mem)))
        .collect();

    let mut d = SharedData {
        spawns,
        room,
        sources,
        source_alloc,
        construction_alloc,
        role_count,
        energy,
    };

    let mut transport = TransportAllocator::new();

    put_containers(&d);

    let my_structures = d.room.find(find::MY_STRUCTURES, None);

    my_structures
        .iter()
        .cloned()
        .filter_map(|s| -> Option<StructureTower> { s.try_into().ok() })
        .for_each(|tower| tower::run(tower, &mut transport));

    for (_, memory) in &creep_mems {
        match memory.discriminant() {
            RoleType::Hauler => d.role_count.haulers += 1,
            RoleType::Harvester => d.role_count.harvesters += 1,
            RoleType::Upgrader => d.role_count.upgraders += 1,
            RoleType::Builder => d.role_count.builders += 1,
        }
    }

    // Register stage
    for (creep, memory) in &mut creep_mems {
        memory.register(creep, &mut d, &mut transport);
    }

    // Add source containers as exporters
    d.sources
        .iter()
        .filter_map(|s| match s.container {
            ContainerInfo::Built(id) => Some(id),
            _ => None,
        })
        .filter_map(|id| id.resolve())
        .filter(|c| c.store().get(ResourceType::Energy).unwrap_or(0) > 0)
        .for_each(|c| transport.register_export(c, Priority::High));

    let storage: Option<StructureStorage> = my_structures
        .iter()
        .cloned()
        .find_map(|s| s.try_into().ok());
    if let Some(s) = storage {
        transport.register_import(s.clone(), Priority::Passive);
        transport.register_export(s, Priority::Passive);
    }

    d.spawns.iter().cloned().for_each(|s| {
        transport.register_import(s, Priority::High);
    });

    my_structures
        .iter()
        .cloned()
        .filter_map(|s| -> Option<StructureExtension> { s.try_into().ok() })
        .for_each(|s| {
            transport.register_import(s, Priority::High);
        });

    // Allocation stage
    let transport = transport.allocate();
    d.construction_alloc.allocate();

    let mut path_finder = PathFinder::new(creep_mems.iter().map(|(creep, _)| creep));

    // Execute stage
    for (creep, mut memory) in creep_mems {
        memory.run(
            &creep,
            &mut d,
            &mut room_memory,
            &mut path_finder,
            &transport,
        );

        creep.set_memory(&to_value(&memory).expect("Failed to serialize memory"));
    }

    // Path find stage
    let mut idle_keepouts = Vec::new();
    for spawn in &d.spawns {
        idle_keepouts.push(SearchGoal::new(spawn.pos(), 5));
    }
    for source in &d.sources {
        idle_keepouts.push(SearchGoal::new(source.source.pos(), 1));
    }
    path_finder.process_movements(idle_keepouts);

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
}
