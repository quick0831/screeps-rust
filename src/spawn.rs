use std::cmp::max;
use std::cmp::min;

use log::error;
use log::info;
use screeps::Part;
use screeps::ResourceType;
use screeps::StructureSpawn;
use screeps::TextAlign;
use screeps::TextStyle;
use screeps::game;
use screeps::prelude::*;
use strum::IntoDiscriminant as _;

use crate::ShardMemory;
use crate::roles::*;
use crate::room::SharedData;
use crate::source::ContainerInfo;

pub fn process_spawning(d: &SharedData, memory: &mut ShardMemory) {
    let time = game::time();

    let show_text = |spawn: &StructureSpawn, text: String| {
        let visual = d.room.visual();
        let pos = spawn.pos();
        let style = TextStyle::default().align(TextAlign::Left);
        visual.text(pos.x().u8() as f32, pos.y().u8() as f32, text, Some(style));
    };

    let container_energy: u32 = d
        .sources
        .iter()
        .filter_map(|s| match s.container {
            ContainerInfo::Built(id) => id.resolve()?.store().get(ResourceType::Energy),
            _ => None,
        })
        .sum();

    let cold_start = d.role_count.harvesters == 0 || d.role_count.haulers == 0;

    let harvester_unit_part = [Part::Move, Part::Work, Part::Carry];
    let harvester_unit_cost: u32 = harvester_unit_part.map(Part::cost).into_iter().sum();
    let harvester_spawn_cap = (d.energy.capacity / harvester_unit_cost) as u8;
    let harvester_spawn_cap = if cold_start { 1 } else { harvester_spawn_cap };

    for spawn in &d.spawns {
        let body;
        let name;
        let mem;
        let cost;

        if let Some(spawning) = spawn.spawning() {
            if let Some(name) = spawning.name().as_string()
                && let Some(mem) = memory.creeps.get(&name)
            {
                let role = mem.discriminant();
                show_text(spawn, format!("🛠️ {role}"));
            }
            continue;
        } else if container_energy > 300 && d.role_count.haulers < 5 {
            let unit_part = [Part::Move, Part::Carry];
            let unit_cost: u32 = unit_part.map(Part::cost).into_iter().sum();
            let spawn_cap = (max(300, d.energy.capacity - 300) / unit_cost) as u8;
            let spawn_cap = if cold_start { 2 } else { spawn_cap };
            let spawn_size = min(10, spawn_cap) as usize;
            cost = unit_cost * spawn_size as u32;
            body = unit_part.repeat(spawn_size);
            name = format!("Hauler{time}");
            mem = Hauler::default().into();
        } else if let Some((target, spawn_size)) =
            d.source_alloc.get_creep_spawn_info(harvester_spawn_cap)
        {
            cost = harvester_unit_cost * spawn_size as u32;
            body = harvester_unit_part.repeat(spawn_size as usize);
            name = format!("Harvester{time}");
            mem = Harvester::new(target).into();
        } else if d.construction_alloc.get_creep_spawn_info(&d.role_count) {
            body = vec![Part::Move, Part::Move, Part::Work, Part::Carry];
            cost = body.iter().cloned().map(Part::cost).sum();
            name = format!("Builder{time}");
            mem = Builder::default().into();
        } else if d.role_count.upgraders < 3 && d.energy.capacity - d.energy.available < 100 {
            let unit_part = [Part::Move, Part::Move, Part::Work, Part::Carry];
            let unit_cost: u32 = unit_part.map(Part::cost).into_iter().sum();
            let spawn_cap = (max(300, d.energy.capacity - 300) / unit_cost) as u8;
            let spawn_size = min(5, spawn_cap) as usize;
            cost = unit_cost * spawn_size as u32;
            body = unit_part.repeat(spawn_size);
            name = format!("Upgrader{time}");
            mem = Upgrader::default().into();
        } else {
            break;
        }

        if d.energy.available >= cost {
            if let Err(err) = spawn.spawn_creep(&body, &name) {
                error!("Spawning error: {err}");
            } else {
                info!("Spawning: {name}");
                memory.creeps.insert(name, mem);
            }
        } else {
            show_text(spawn, format!("Need {}/{}", d.energy.available, cost));
        }

        break;
    }
}
