use log::warn;
use screeps::RoomXY;
use screeps::StructureType;
use screeps::Terrain;
use screeps::look::LookResult;

use crate::room::SharedData;
use crate::source::ContainerInfo;

pub fn put_containers(d: &SharedData) {
    for source in &d.sources {
        if let ContainerInfo::None = source.container {
            // Find a place to put container

            let area: Vec<_> = source
                .nearby_area
                .iter()
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

            let Some(pos) = new_container_pos else {
                continue;
            };

            if let Err(err) = d.room.create_construction_site(
                pos.x.u8(),
                pos.y.u8(),
                StructureType::Container,
                None,
            ) {
                warn!("Failed to place construction site for container ({pos}): {err}");
            }
        }
    }
}
