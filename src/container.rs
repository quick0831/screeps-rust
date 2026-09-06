use log::warn;
use screeps::RoomXY;
use screeps::StructureContainer;
use screeps::StructureType;
use screeps::Terrain;
use screeps::find;
use screeps::look::LookResult;
use screeps::prelude::*;

use crate::SharedData;

pub fn put_containers(d: &SharedData) {
    let unbuilt_containers = d
        .room
        .find(find::CONSTRUCTION_SITES, None)
        .into_iter()
        .filter(|e| e.structure_type() == StructureType::Container)
        .map(|e| e.pos());

    let built_containers = d
        .room
        .find(find::STRUCTURES, None)
        .into_iter()
        .filter_map(|s| -> Option<StructureContainer> { s.try_into().ok() })
        .map(|e| e.pos());

    let containers: Vec<_> = unbuilt_containers.chain(built_containers).collect();

    for source in &d.sources {
        let source_pos = source.pos();

        let has_container = containers
            .iter()
            .find(|e| e.in_range_to(source_pos, 1))
            .is_some();

        if !has_container {
            // Find a place to put container
            let xy = source_pos.xy();
            let x = xy.x.u8();
            let y = xy.y.u8();

            let area: Vec<_> = d
                .room
                .look_at_area(y - 1, x - 1, y + 1, x + 1)
                .into_iter()
                .filter(|p| {
                    matches!(
                        p.look_result,
                        LookResult::Terrain(Terrain::Plain | Terrain::Swamp)
                    )
                })
                .map(|p| RoomXY::checked_new(p.x, p.y).expect("Result out of bound"))
                .collect();

            let new_container_pos = area
                .iter()
                .max_by_key(|a| area.iter().filter(|b| a.is_near_to(**b)).count())
                .copied();

            if let Some(pos) = new_container_pos
                && let Err(err) = d.room.create_construction_site(
                    pos.x.u8(),
                    pos.y.u8(),
                    StructureType::Container,
                    None,
                )
            {
                warn!("Failed to place construction site for container ({pos}): {err}");
            }
        }
    }
}
