use log::warn;
use screeps::RoomXY;
use screeps::StructureType;
use screeps::Terrain;
use screeps::look::LookResult;
use screeps::prelude::*;

use crate::SharedData;

pub fn put_containers(d: &SharedData) {
    for source in &d.sources {
        let pos = source.pos().xy();
        let x = pos.x.u8();
        let y = pos.y.u8();

        let area: Vec<_> = d.room.look_at_area(y - 1, x - 1, y + 1, x + 1);

        let has_container = area
            .iter()
            .find(|p| match &p.look_result {
                LookResult::Structure(s) => s.structure_type() == StructureType::Container,
                LookResult::ConstructionSite(s) => s.structure_type() == StructureType::Container,
                _ => false,
            })
            .is_some();

        if !has_container {
            // Find a place to put container

            let area: Vec<_> = area
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
