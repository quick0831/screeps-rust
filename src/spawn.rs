use std::cmp::max;
use std::cmp::min;

use log::error;
use log::info;
use screeps::Part;
use screeps::SpawnOptions;
use screeps::TextAlign;
use screeps::TextStyle;
use screeps::find;
use screeps::game;
use screeps::prelude::*;
use serde_wasm_bindgen::from_value;
use serde_wasm_bindgen::to_value;
use strum::IntoDiscriminant as _;

use crate::SharedData;
use crate::roles::*;

pub fn process_spawning(d: &SharedData) {
    let time = game::time();
    let harvester_spawn_size = d.source_alloc.get_creep_spawn_size();
    let has_construction_sites = !d.room.find(find::CONSTRUCTION_SITES, None).is_empty();

    let spawn_creep = |body: &[Part], name: &str, mem: &Role| {
        let cost: u32 = body.iter().map(|p| p.cost()).sum();
        if d.energy.available >= cost {
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
    } else if d.role_count.haulers < 3 && d.role_count.harvesters >= 1 {
        let unit_part = [Part::Move, Part::Carry];
        let unit_cost: u32 = unit_part.map(Part::cost).into_iter().sum();
        let spawn_cap = (max(300, d.energy.capacity - 300) / unit_cost) as u8;
        let spawn_size = min(6, spawn_cap) as usize;
        let body = unit_part.repeat(spawn_size);
        let name = format!("Hauler{time}");
        let mem = Hauler::default().into();
        spawn_creep(&body, &name, &mem);
    } else if harvester_spawn_size != 0 {
        let unit_part = [Part::Move, Part::Work, Part::Carry];
        let unit_cost: u32 = unit_part.map(Part::cost).into_iter().sum();
        let spawn_cap = (max(300, d.energy.capacity - 300) / unit_cost) as u8;
        let spawn_size = min(harvester_spawn_size, spawn_cap) as usize;
        let body = unit_part.repeat(spawn_size);
        let name = format!("Harvester{time}");
        let mem = Harvester::default().into();
        spawn_creep(&body, &name, &mem);
    } else if d.role_count.builders < 2 && has_construction_sites && d.role_count.upgraders != 0 {
        let body = vec![Part::Move, Part::Move, Part::Work, Part::Carry];
        let name = format!("Builder{time}");
        let mem = Builder::default().into();
        spawn_creep(&body, &name, &mem);
    } else if d.energy.capacity - d.energy.available < 100 {
        let unit_part = [Part::Move, Part::Move, Part::Work, Part::Carry];
        let unit_cost: u32 = unit_part.map(Part::cost).into_iter().sum();
        let spawn_cap = (max(300, d.energy.capacity - 300) / unit_cost) as u8;
        let spawn_size = min(6, spawn_cap) as usize;
        let body = unit_part.repeat(spawn_size);
        let name = format!("Upgrader{time}");
        let mem = Upgrader::default().into();
        spawn_creep(&body, &name, &mem);
    }
}
